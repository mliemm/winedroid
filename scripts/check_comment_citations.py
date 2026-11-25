import argparse
import ast
import bisect
import io
import os
import re
import subprocess
import sys
import tokenize
from dataclasses import dataclass
from pathlib import Path


ALLOWED_REPOSITORIES = (
    "androidx",
    "frameworks-base",
    "libcore",
    "art",
    "dalvik",
    "bionic",
    "logging",
    "frameworks-native",
    "cts",
)

C_LIKE_LANGUAGES = {
    ".c": "c",
    ".h": "c",
    ".java": "java",
    ".rs": "rust",
}

BINARY_SUFFIXES = {
    ".dex",
    ".png",
    ".so",
    ".ttf",
}

DATA_SUFFIXES = {
    ".json",
    ".txt",
}

GENERATED_FILENAMES = {
    "Cargo.lock",
}

LEGAL_FILENAMES = {
    "LICENSE",
}

@dataclass(frozen=True)
class Violation:
    path: Path
    line: int
    column: int
    reason: str
    excerpt: str


@dataclass(frozen=True)
class Citation:
    path: Path
    line: int
    column: int
    repository: str
    filename: str
    anchors: tuple[str, ...]
    excerpt: str


def line_starts(source: str) -> list[int]:
    starts = [0]
    for index, character in enumerate(source):
        if character == "\n":
            starts.append(index + 1)
    return starts


def line_position(starts: list[int], index: int) -> tuple[int, int]:
    line_index = bisect.bisect_right(starts, index) - 1
    return line_index + 1, index - starts[line_index] + 1


def source_line(source: str, starts: list[int], line: int) -> str:
    start = starts[line - 1]
    end = source.find("\n", start)
    if end == -1:
        end = len(source)
    return source[start:end].rstrip("\r")


def translate_java_unicode(source: str) -> tuple[str, list[int]]:
    translated = []
    origins = []
    cursor = 0
    trailing_backslashes = 0
    last_character_from_escape = False

    while cursor < len(source):
        character = source[cursor]
        eligible = character == "\\" and (
            not translated or last_character_from_escape or trailing_backslashes % 2 == 0
        )
        match = None
        if eligible:
            match = re.match(r"\\u+([0-9a-fA-F]{4})", source[cursor:])

        if match is not None:
            character = chr(int(match.group(1), 16))
            translated.append(character)
            origins.append(cursor)
            cursor += match.end()
            last_character_from_escape = True
        else:
            translated.append(character)
            origins.append(cursor)
            cursor += 1
            last_character_from_escape = False

        if character == "\\":
            trailing_backslashes += 1
        else:
            trailing_backslashes = 0

    return "".join(translated), origins


def rust_raw_string_end(source: str, index: int) -> int | None:
    if index > 0 and (source[index - 1].isalnum() or source[index - 1] == "_"):
        return None

    delimiter_start = None
    if source.startswith("br", index) or source.startswith("cr", index):
        delimiter_start = index + 2
    elif source.startswith("r", index):
        delimiter_start = index + 1
    if delimiter_start is None:
        return None

    quote_index = delimiter_start
    while quote_index < len(source) and source[quote_index] == "#":
        quote_index += 1
    if quote_index >= len(source) or source[quote_index] != '"':
        return None

    hashes = source[delimiter_start:quote_index]
    closing = '"' + hashes
    end = source.find(closing, quote_index + 1)
    if end == -1:
        return len(source)
    return end + len(closing)


def quoted_string_end(source: str, index: int, quote: str) -> int:
    cursor = index + 1
    while cursor < len(source):
        character = source[cursor]
        if character == "\\":
            cursor += 2
            continue
        if character == quote:
            return cursor + 1
        cursor += 1
    return len(source)


def java_text_block_end(source: str, index: int) -> int:
    cursor = index + 3
    while cursor < len(source):
        closing = source.find('"""', cursor)
        if closing == -1:
            return len(source)
        backslashes = 0
        check = closing - 1
        while check >= 0 and source[check] == "\\":
            backslashes += 1
            check -= 1
        if backslashes % 2 == 0:
            return closing + 3
        cursor = closing + 3
    return len(source)


def character_literal_end(source: str, index: int) -> int | None:
    cursor = index + 1
    if cursor >= len(source) or source[cursor] in "\r\n'":
        return None

    if source[cursor] == "\\":
        cursor += 1
        if cursor >= len(source):
            return None
        escape = source[cursor]
        cursor += 1
        if escape == "x":
            cursor += 2
        elif escape == "u" and cursor < len(source) and source[cursor] == "{":
            closing = source.find("}", cursor + 1)
            if closing == -1:
                return None
            cursor = closing + 1
    else:
        cursor += 1

    if cursor < len(source) and source[cursor] == "'":
        return cursor + 1
    return None


def block_comment_end(source: str, index: int, nested: bool) -> int:
    depth = 1
    cursor = index + 2
    while cursor < len(source):
        if nested and source.startswith("/*", cursor):
            depth += 1
            cursor += 2
            continue
        if source.startswith("*/", cursor):
            depth -= 1
            cursor += 2
            if depth == 0:
                return cursor
            continue
        cursor += 1
    return len(source)


def parse_citation(
    path: Path,
    line: int,
    column: int,
    full_line: str,
    comment: str,
) -> tuple[Citation | None, Violation | None]:
    prefix = full_line[: column - 1]
    if prefix.strip():
        return None, Violation(
            path,
            line,
            column,
            "comments must be standalone citation lines",
            full_line,
        )

    if comment.startswith("///") or comment.startswith("//!"):
        return None, Violation(path, line, column, "doc comments are not allowed", full_line)
    if not comment.startswith("// cite: "):
        return None, Violation(
            path,
            line,
            column,
            "only citation line comments are allowed",
            full_line,
        )

    body = comment[len("// cite: ") :]
    if " " not in body:
        return None, Violation(path, line, column, "citation is missing its source target", full_line)

    repository, target = body.split(" ", 1)
    if repository not in ALLOWED_REPOSITORIES:
        return None, Violation(path, line, column, "citation repository is not allowed", full_line)
    if target.startswith(" ") or target.count("#") != 1:
        return None, Violation(
            path,
            line,
            column,
            "citation must contain exactly one FileName#anchor target",
            full_line,
        )

    filename, anchor_text = target.split("#", 1)
    if not filename or any(character.isspace() for character in filename):
        return None, Violation(path, line, column, "citation filename is missing or malformed", full_line)
    if "/" in filename or "\\" in filename:
        return None, Violation(path, line, column, "citation filename must be bare", full_line)
    if not anchor_text or anchor_text != anchor_text.strip() or anchor_text.endswith(","):
        return None, Violation(path, line, column, "citation anchor list is empty or malformed", full_line)
    if " -- " in anchor_text or re.search(r"\s\(", anchor_text):
        return None, Violation(path, line, column, "citation carries explanatory prose", full_line)

    anchors = tuple(anchor_text.split(", "))
    if any("," in anchor for anchor in anchors):
        return None, Violation(path, line, column, "citation anchors must use canonical comma-space separators", full_line)
    if len(set(anchors)) != len(anchors):
        return None, Violation(path, line, column, "citation repeats an anchor", full_line)
    for anchor in anchors:
        if not anchor or any(character.isspace() for character in anchor):
            return None, Violation(path, line, column, "citation anchor must be a name", full_line)
        if any(character in anchor for character in "()#,\\;"):
            return None, Violation(path, line, column, "citation anchor must be a name, not a signature or path", full_line)
        if anchor.isdecimal():
            return None, Violation(path, line, column, "citation anchor cannot be a line number", full_line)

    citation = Citation(path, line, column, repository, filename, anchors, full_line)
    return citation, None


def scan_c_like(path: Path, language: str) -> tuple[list[Violation], list[Citation]]:
    source = path.read_text(encoding="utf-8", errors="replace")
    lexed_source = source
    origins = None
    if language == "java":
        lexed_source, origins = translate_java_unicode(source)
    starts = line_starts(source)
    violations = []
    citations = []
    cursor = 0

    while cursor < len(lexed_source):
        if language == "rust":
            raw_end = rust_raw_string_end(lexed_source, cursor)
            if raw_end is not None:
                cursor = raw_end
                continue

        if language == "java" and lexed_source.startswith('"""', cursor):
            cursor = java_text_block_end(lexed_source, cursor)
            continue

        character = lexed_source[cursor]
        if character == '"':
            cursor = quoted_string_end(lexed_source, cursor, '"')
            continue
        if character == "'":
            literal_end = character_literal_end(lexed_source, cursor)
            if literal_end is not None:
                cursor = literal_end
                continue

        if lexed_source.startswith("//", cursor):
            original_cursor = cursor if origins is None else origins[cursor]
            line, column = line_position(starts, original_cursor)
            end = lexed_source.find("\n", cursor)
            if end == -1:
                end = len(lexed_source)
            full_line = source_line(source, starts, line)
            original_end = source.find("\n", original_cursor)
            if original_end == -1:
                original_end = len(source)
            comment = source[original_cursor:original_end].rstrip("\r")
            citation, violation = parse_citation(path, line, column, full_line, comment)
            if citation is not None:
                citations.append(citation)
            if violation is not None:
                violations.append(violation)
            cursor = end
            continue

        if lexed_source.startswith("/*", cursor):
            original_cursor = cursor if origins is None else origins[cursor]
            line, column = line_position(starts, original_cursor)
            violations.append(
                Violation(
                    path,
                    line,
                    column,
                    "block comments are not allowed",
                    source_line(source, starts, line),
                )
            )
            cursor = block_comment_end(lexed_source, cursor, language == "rust")
            continue

        cursor += 1

    return violations, citations


def violation_at(path: Path, source: str, starts: list[int], index: int, reason: str) -> Violation:
    line, column = line_position(starts, index)
    return Violation(path, line, column, reason, source_line(source, starts, line))


def scan_python(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []

    try:
        tokens = tokenize.generate_tokens(io.StringIO(source).readline)
        for token in tokens:
            if token.type != tokenize.COMMENT:
                continue
            line, column = token.start
            if line == 1 and column == 0 and token.string.startswith("#!"):
                continue
            violations.append(
                Violation(
                    path,
                    line,
                    column + 1,
                    "Python comments are not allowed",
                    source_line(source, starts, line),
                )
            )
    except (IndentationError, tokenize.TokenError) as error:
        line = getattr(error, "lineno", 1) or 1
        violations.append(
            Violation(
                path,
                line,
                1,
                "cannot tokenize Python source",
                source_line(source, starts, line),
            )
        )

    try:
        tree = ast.parse(source, filename=str(path))
    except SyntaxError as error:
        line = error.lineno or 1
        violations.append(
            Violation(
                path,
                line,
                error.offset or 1,
                "cannot parse Python source",
                source_line(source, starts, line),
            )
        )
        return violations

    docstring_owners = (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)
    for owner in ast.walk(tree):
        if not isinstance(owner, docstring_owners) or not owner.body:
            continue
        expression = owner.body[0]
        if not isinstance(expression, ast.Expr):
            continue
        value = expression.value
        if not isinstance(value, ast.Constant) or not isinstance(value.value, str):
            continue
        violations.append(
            Violation(
                path,
                expression.lineno,
                expression.col_offset + 1,
                "Python docstrings are not allowed",
                source_line(source, starts, expression.lineno),
            )
        )

    return violations


def scan_shell(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []
    cursor = 0

    while cursor < len(source):
        if cursor == 0 and source.startswith("#!"):
            newline = source.find("\n", cursor)
            cursor = len(source) if newline == -1 else newline + 1
            continue
        character = source[cursor]
        if character in "\"'":
            cursor = quoted_string_end(source, cursor, character)
            continue
        if character == "\\":
            cursor += 2
            continue
        if character == "#" and (
            cursor == 0 or source[cursor - 1] in " \t\r\n;|&()<>"
        ):
            violations.append(
                violation_at(path, source, starts, cursor, "shell comments are not allowed")
            )
            newline = source.find("\n", cursor)
            cursor = len(source) if newline == -1 else newline + 1
            continue
        cursor += 1

    return violations


def toml_string_end(source: str, index: int) -> int:
    quote = source[index]
    delimiter = quote * 3
    if source.startswith(delimiter, index):
        cursor = index + 3
        while cursor < len(source):
            if source.startswith(delimiter, cursor):
                return cursor + 3
            if quote == '"' and source[cursor] == "\\":
                cursor += 2
            else:
                cursor += 1
        return len(source)
    return quoted_string_end(source, index, quote)


def scan_toml(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []
    cursor = 0

    while cursor < len(source):
        character = source[cursor]
        if character in "\"'":
            cursor = toml_string_end(source, cursor)
            continue
        if character == "#":
            violations.append(
                violation_at(path, source, starts, cursor, "TOML comments are not allowed")
            )
            newline = source.find("\n", cursor)
            cursor = len(source) if newline == -1 else newline + 1
            continue
        cursor += 1

    return violations


def scan_javascript(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []

    def scan_template(cursor: int) -> int:
        cursor += 1
        while cursor < len(source):
            if source[cursor] == "\\":
                cursor += 2
                continue
            if source[cursor] == "`":
                return cursor + 1
            if source.startswith("${", cursor):
                cursor = scan_code(cursor + 2, True)
                continue
            cursor += 1
        return len(source)

    def scan_code(cursor: int, stop_at_brace: bool = False) -> int:
        brace_depth = 0
        while cursor < len(source):
            character = source[cursor]
            if character in "\"'":
                cursor = quoted_string_end(source, cursor, character)
                continue
            if character == "\\":
                cursor += 2
                continue
            if character == "`":
                cursor = scan_template(cursor)
                continue
            if source.startswith("//", cursor):
                violations.append(
                    violation_at(
                        path,
                        source,
                        starts,
                        cursor,
                        "JavaScript line comments are not allowed",
                    )
                )
                newline = source.find("\n", cursor)
                cursor = len(source) if newline == -1 else newline + 1
                continue
            if source.startswith("/*", cursor):
                violations.append(
                    violation_at(
                        path,
                        source,
                        starts,
                        cursor,
                        "JavaScript block comments are not allowed",
                    )
                )
                closing = source.find("*/", cursor + 2)
                cursor = len(source) if closing == -1 else closing + 2
                continue
            if stop_at_brace:
                if character == "{":
                    brace_depth += 1
                elif character == "}":
                    if brace_depth == 0:
                        return cursor + 1
                    brace_depth -= 1
            cursor += 1
        return len(source)

    scan_code(0)
    return violations


def scan_css(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []
    cursor = 0

    while cursor < len(source):
        character = source[cursor]
        if character in "\"'":
            cursor = quoted_string_end(source, cursor, character)
            continue
        if source.startswith("/*", cursor):
            violations.append(
                violation_at(path, source, starts, cursor, "CSS block comments are not allowed")
            )
            closing = source.find("*/", cursor + 2)
            cursor = len(source) if closing == -1 else closing + 2
            continue
        cursor += 1

    return violations


def scan_automation(path: Path) -> list[Violation]:
    violations = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        column = line.find("#")
        if column != -1:
            violations.append(
                Violation(path, line_number, column + 1, "automation comments are not allowed", line)
            )
    return violations


def scan_markup(path: Path) -> list[Violation]:
    source = path.read_text(encoding="utf-8", errors="replace")
    starts = line_starts(source)
    violations = []
    cursor = 0
    while cursor < len(source):
        cdata = source.find("<![CDATA[", cursor)
        comment = source.find("<!--", cursor)
        if comment == -1:
            break
        if cdata != -1 and cdata < comment:
            closing = source.find("]]>", cdata + 9)
            if closing == -1:
                break
            cursor = closing + 3
            continue
        line, column = line_position(starts, comment)
        violations.append(
            Violation(
                path,
                line,
                column,
                "markup comments are not allowed",
                source_line(source, starts, line),
            )
        )
        closing = source.find("-->", comment + 4)
        cursor = len(source) if closing == -1 else closing + 3
    return violations


def collect_files(roots: list[Path]) -> list[Path]:
    files = []
    for root in roots:
        if root.is_file():
            files.append(root)
            continue
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if path.is_file() and "__pycache__" not in path.parts:
                files.append(path)
    return sorted(files)


def collect_repository_files(repository_root: Path) -> list[Path]:
    result = subprocess.run(
        [
            "git",
            "-C",
            str(repository_root),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        error = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"cannot list repository files: {error}")

    files = []
    for encoded in result.stdout.split(b"\0"):
        if not encoded:
            continue
        path = repository_root / os.fsdecode(encoded)
        if path.is_file():
            files.append(path)
    return sorted(files)


class ReferenceIndex:
    def __init__(self, root: Path):
        self.root = root
        self.paths_by_repository = {}
        self.contents = {}

    def load_repository(self, repository: str) -> dict[str, list[Path]]:
        cached = self.paths_by_repository.get(repository)
        if cached is not None:
            return cached

        checkout = self.root / repository
        if not checkout.is_dir():
            raise RuntimeError(f"reference checkout is missing: {checkout}")

        paths = []
        if (checkout / ".git").exists():
            environment = os.environ.copy()
            environment["GIT_NO_LAZY_FETCH"] = "1"
            result = subprocess.run(
                ["git", "-C", str(checkout), "ls-files", "-z"],
                check=False,
                capture_output=True,
                env=environment,
            )
            if result.returncode != 0:
                error = result.stderr.decode("utf-8", errors="replace").strip()
                raise RuntimeError(f"cannot read reference index {checkout}: {error}")
            for encoded in result.stdout.split(b"\0"):
                if encoded:
                    paths.append(checkout / os.fsdecode(encoded))
        else:
            paths.extend(path for path in checkout.rglob("*") if path.is_file())

        by_filename = {}
        for path in paths:
            if not path.is_file():
                continue
            by_filename.setdefault(path.name, []).append(path)
        self.paths_by_repository[repository] = by_filename
        return by_filename

    def read(self, path: Path) -> str:
        cached = self.contents.get(path)
        if cached is None:
            cached = path.read_text(encoding="utf-8", errors="replace")
            self.contents[path] = cached
        return cached


def validate_citation_blocks(citations: list[Citation]) -> list[Violation]:
    violations = []
    previous_path = None
    previous_line = -1
    seen = set()

    for citation in sorted(citations, key=lambda item: (str(item.path), item.line)):
        if citation.path != previous_path or citation.line != previous_line + 1:
            seen = set()
        key = (citation.repository, citation.filename)
        if key in seen:
            violations.append(
                Violation(
                    citation.path,
                    citation.line,
                    citation.column,
                    "a consecutive citation block must cite each source file only once",
                    citation.excerpt,
                )
            )
        seen.add(key)
        previous_path = citation.path
        previous_line = citation.line

    return violations


def validate_reference_sources(
    citations: list[Citation],
    reference_root: Path,
) -> list[Violation]:
    violations = []
    index = ReferenceIndex(reference_root)
    unavailable = {}

    for repository in sorted({citation.repository for citation in citations}):
        try:
            index.load_repository(repository)
        except RuntimeError as error:
            unavailable[repository] = str(error)

    for repository, reason in unavailable.items():
        violations.append(Violation(reference_root / repository, 0, 0, reason, ""))

    for citation in citations:
        if citation.repository in unavailable:
            continue
        candidates = index.load_repository(citation.repository).get(citation.filename, [])
        if not candidates:
            violations.append(
                Violation(
                    citation.path,
                    citation.line,
                    citation.column,
                    "citation source filename is not physically present in its reference checkout",
                    citation.excerpt,
                )
            )
            continue

        matching = False
        candidate_contents = []
        for candidate in candidates:
            try:
                contents = index.read(candidate)
            except OSError:
                continue
            candidate_contents.append(contents)
            if all(anchor_is_findable(anchor, contents) for anchor in citation.anchors):
                matching = True
                break
        if not matching:
            missing = [
                anchor
                for anchor in citation.anchors
                if not any(anchor_is_findable(anchor, contents) for contents in candidate_contents)
            ]
            if missing:
                reason = (
                    f"citation anchors are not findable in {citation.repository} "
                    f"{citation.filename}: {', '.join(missing)}"
                )
            else:
                reason = (
                    f"citation anchors do not resolve together in one "
                    f"{citation.repository} {citation.filename} source file"
                )
            violations.append(
                Violation(
                    citation.path,
                    citation.line,
                    citation.column,
                    reason,
                    citation.excerpt,
                )
            )

    return violations


def anchor_is_findable(anchor: str, contents: str) -> bool:
    prefix = r"(?<![\w$])" if anchor[0].isalnum() or anchor[0] in "_$" else ""
    suffix = r"(?![\w$])" if anchor[-1].isalnum() or anchor[-1] in "_$" else ""
    return re.search(prefix + re.escape(anchor) + suffix, contents) is not None


def scan_gitignore(path: Path) -> list[Violation]:
    violations = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        if line.startswith("#"):
            violations.append(
                Violation(path, line_number, 1, "gitignore comments are not allowed", line)
            )
    return violations


def scan_files(files: list[Path], reference_root: Path) -> tuple[list[Violation], list[Citation]]:
    violations = []
    citations = []
    for path in files:
        suffix = path.suffix.lower()
        if path.name in GENERATED_FILENAMES | LEGAL_FILENAMES:
            continue
        language = C_LIKE_LANGUAGES.get(suffix)
        if language is not None:
            found_violations, found_citations = scan_c_like(path, language)
            violations.extend(found_violations)
            citations.extend(found_citations)
        elif suffix == ".py":
            violations.extend(scan_python(path))
        elif suffix == ".sh":
            violations.extend(scan_shell(path))
        elif suffix == ".toml":
            violations.extend(scan_toml(path))
        elif suffix in {".js", ".mjs"}:
            violations.extend(scan_javascript(path))
        elif suffix == ".css":
            violations.extend(scan_css(path))
        elif suffix == ".auto":
            violations.extend(scan_automation(path))
        elif suffix in {".html", ".md", ".xml"}:
            violations.extend(scan_markup(path))
        elif path.name == ".gitignore":
            violations.extend(scan_gitignore(path))
        elif suffix not in BINARY_SUFFIXES | DATA_SUFFIXES:
            violations.append(
                Violation(
                    path,
                    1,
                    1,
                    "comment policy checker does not support this source/test file type",
                    "",
                )
            )

    violations.extend(validate_citation_blocks(citations))
    violations.extend(validate_reference_sources(citations, reference_root))
    violations.sort(key=lambda item: (str(item.path), item.line, item.column, item.reason))
    return violations, citations


def scan_paths(roots: list[Path], reference_root: Path) -> tuple[list[Violation], list[Citation]]:
    return scan_files(collect_files(roots), reference_root)


def scan_repository(
    repository_root: Path,
    reference_root: Path,
) -> tuple[list[Violation], list[Citation]]:
    return scan_files(collect_repository_files(repository_root), reference_root)


def write_failure_report(violations: list[Violation], stream) -> None:
    print("comment citation policy violations:", file=stream)
    for violation in violations:
        location = str(violation.path)
        if violation.line:
            location += f":{violation.line}:{violation.column}"
        print(f"  {location}: {violation.reason}", file=stream)
        if violation.excerpt:
            print(f"    {violation.excerpt.strip()}", file=stream)

    print("", file=stream)
    print("Only permitted comment form:", file=stream)
    print("  // cite: <repo> <FileName>#<anchor>[, <anchor>...]", file=stream)
    print(f"Allowed repositories: {', '.join(ALLOWED_REPOSITORIES)}", file=stream)
    print("Use one bare source filename per cite line and consecutive cite lines for multiple files.", file=stream)
    print("Anchors must be literal method, field, constant, or attribute names present in that file.", file=stream)
    print("Not allowed:", file=stream)
    print("  explanatory or descriptive comments", file=stream)
    print("  TODO / FIXME / NOTE / HACK / XXX comments", file=stream)
    print("  doc comments or block comments", file=stream)
    print("  commented-out code", file=stream)
    print("  narration, section banners, or separators", file=stream)
    print("  line-number anchors", file=stream)
    print("  embedded citations, trailing prose, signatures, paths, or multiple source files", file=stream)


def parse_arguments(arguments: list[str]):
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--android-ref",
        type=Path,
        default=Path(os.environ.get("ANDROID_REF_ROOT", "~/Dev/android-ref")).expanduser(),
    )
    parser.add_argument("--repository", action="store_true")
    parser.add_argument("--repository-root", type=Path, default=Path("."))
    parser.add_argument("roots", nargs="*", type=Path)
    options = parser.parse_args(arguments)
    if options.repository and options.roots:
        parser.error("--repository cannot be combined with explicit roots")
    return options


def main(arguments: list[str]) -> int:
    options = parse_arguments(arguments)
    try:
        if options.repository:
            violations, citations = scan_repository(
                options.repository_root,
                options.android_ref,
            )
        else:
            roots = options.roots or [Path("src"), Path("tests")]
            violations, citations = scan_paths(roots, options.android_ref)
    except RuntimeError as error:
        print(f"comment citation policy check failed: {error}", file=sys.stderr)
        return 1
    if violations:
        write_failure_report(violations, sys.stderr)
        return 1
    print(f"comment citation check passed ({len(citations)} citations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
