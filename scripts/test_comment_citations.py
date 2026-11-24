import io
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

import check_comment_citations as checker


def write(path: Path, contents: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


def fake_reference(root: Path) -> None:
    write(root / "frameworks-base" / "Framework.java", "void method() {}\nint FIELD;\nint value123;\n")
    write(root / "frameworks-base" / "Substring.java", "void methodSuffix() {}\n")
    write(root / "frameworks-base" / "screen.xml", '<item name="@android:id/content"/>\n')
    for repository in checker.ALLOWED_REPOSITORIES:
        (root / repository).mkdir(parents=True, exist_ok=True)


def reasons(violations: list[checker.Violation]) -> list[str]:
    return [violation.reason for violation in violations]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def test_valid_sources(work: Path, reference: Path) -> None:
    source = work / "src" / "valid.rs"
    write(
        source,
        """// cite: frameworks-base Framework.java#method, FIELD
// cite: frameworks-base screen.xml#@android:id/content
fn valid<'a>(value: &'a str) {
    let url = "https://example.com/path//segment";
    let marker = "//";
    let raw = r#"// cite: nope Bad.java#bad /* hidden */"#;
    let bytes = br##"<!-- hidden --> // hidden"##;
    let slash = '/';
    let _ = (value, url, marker, raw, bytes, slash);
}
""",
    )
    write(
        work / "src" / "Valid.java",
        '// cite: frameworks-base Framework.java#method\nclass Valid { String value = "// not a comment"; String escaped = "\\u002f\\u002f"; String text = """/* hidden */"""; }\n',
    )
    write(work / "tests" / "clean.auto", '0 tap text "7"\n')
    write(work / "tests" / "clean.xml", '<root><![CDATA[<!-- text -->]]></root>\n')
    write(work / "tests" / "NOTES.md", '# Heading\n')
    violations, citations = checker.scan_paths([work / "src", work / "tests"], reference)
    require(not violations, f"valid inputs failed: {reasons(violations)}")
    require(len(citations) == 3, "valid citations were not counted")


def test_comment_forms(work: Path, reference: Path) -> None:
    write(
        work / "src" / "bad_comments.rs",
        """// explanation
/// docs
fn value() {} // cite: frameworks-base Framework.java#method
/* block */
""",
    )
    write(work / "tests" / "bad.auto", '0 tap text "7" # narration\n')
    write(work / "tests" / "bad.xml", '<root><!-- narration --></root>\n')
    write(work / "tests" / "bad.md", '<!-- hidden note -->\n')
    write(work / "tests" / "bad.html", '<main><!-- hidden note --></main>\n')
    write(
        work / "src" / "UnicodeComments.java",
        "class UnicodeComments {\n\\u002f\\u002f hidden\n\\u002f\\u002a block \\u002a\\u002f\n}\n",
    )
    violations, _ = checker.scan_paths([work / "src", work / "tests"], reference)
    found = reasons(violations)
    require("only citation line comments are allowed" in found, "ordinary line comment passed")
    require("doc comments are not allowed" in found, "doc comment passed")
    require("comments must be standalone citation lines" in found, "embedded citation passed")
    require("block comments are not allowed" in found, "block comment passed")
    require(found.count("block comments are not allowed") == 2, "Unicode block comment passed")
    require(found.count("only citation line comments are allowed") == 2, "Unicode line comment passed")
    require("automation comments are not allowed" in found, "automation comment passed")
    require(found.count("markup comments are not allowed") == 3, "markup comment passed")


def test_citation_grammar(work: Path, reference: Path) -> None:
    malformed = [
        "// cite: unknown Framework.java#method",
        "// cite: frameworks-base path/Framework.java#method",
        "// cite: frameworks-base Framework.java",
        "// cite: frameworks-base Framework.java#method,",
        "// cite: frameworks-base Framework.java#method -- explanation",
        "// cite: frameworks-base Framework.java#method(int)",
        "// cite: frameworks-base Framework.java#method,method",
        "// cite: frameworks-base Framework.java#method, method",
        "// cite: frameworks-base Framework.java#123",
    ]
    write(work / "src" / "bad_cites.rs", "\n".join(malformed) + "\n")
    violations, _ = checker.scan_paths([work / "src"], reference)
    found = reasons(violations)
    require(len(violations) == len(malformed), f"expected {len(malformed)} malformed cites: {found}")
    require("citation repository is not allowed" in found, "invalid repository passed")
    require("citation filename must be bare" in found, "qualified filename passed")
    require("citation repeats an anchor" in found, "duplicate anchor passed")
    require("citation anchor cannot be a line number" in found, "line-number anchor passed")


def test_java_citation_spelling(work: Path, reference: Path) -> None:
    write(
        work / "src" / "EscapedCitations.java",
        """\\u002f\\u002f cite: frameworks-base Framework.java#method
// \\u0063ite: frameworks-base Framework.java#method
""",
    )
    violations, citations = checker.scan_paths([work / "src"], reference)
    require(len(violations) == 2, "Unicode-escaped citation spelling passed")
    require(not citations, "Unicode-escaped citations were counted")


def test_repeated_source_block(work: Path, reference: Path) -> None:
    write(
        work / "src" / "repeated.rs",
        """// cite: frameworks-base Framework.java#method
// cite: frameworks-base Framework.java#FIELD
""",
    )
    violations, _ = checker.scan_paths([work / "src"], reference)
    require(
        "a consecutive citation block must cite each source file only once" in reasons(violations),
        "repeated source file passed",
    )


def test_reference_resolution(work: Path, reference: Path) -> None:
    (reference / "frameworks-base" / "Dangling.java").symlink_to("missing-target.java")
    write(
        work / "src" / "bad_sources.rs",
        """// cite: frameworks-base Missing.java#method
// cite: frameworks-base Framework.java#missingAnchor
// cite: frameworks-base Dangling.java#method
""",
    )
    violations, _ = checker.scan_paths([work / "src"], reference)
    found = reasons(violations)
    require(
        "citation source filename is not physically present in its reference checkout" in found,
        "missing filename passed",
    )
    require(any(reason.startswith("citation anchors are not findable") for reason in found), "missing anchor passed")


def test_exact_anchor_resolution(work: Path, reference: Path) -> None:
    write(
        work / "src" / "substring.rs",
        "// cite: frameworks-base Substring.java#method\nfn value() {}\n",
    )
    violations, _ = checker.scan_paths([work / "src"], reference)
    require(
        any(reason.startswith("citation anchors are not findable") for reason in reasons(violations)),
        "anchor resolving only inside a larger identifier passed",
    )


def test_python_comments_and_docstrings(work: Path, reference: Path) -> None:
    write(
        work / "clean.py",
        '''#!/usr/bin/env python3
value = "# data"
payload = """# string data"""
''',
    )
    write(
        work / "bad.py",
        '''"""module docs"""
value = 1  # narration
def function():
    """function docs"""
    return value
''',
    )
    violations, _ = checker.scan_paths([work], reference)
    found = reasons(violations)
    require(found.count("Python comments are not allowed") == 1, "Python comment passed")
    require(found.count("Python docstrings are not allowed") == 2, "Python docstring passed")


def test_shell_comments_and_shebangs(work: Path, reference: Path) -> None:
    write(
        work / "clean.sh",
        '''#!/usr/bin/env bash
value='# data'
trimmed=${value#prefix}
printf '%s\n' "https://example.com/#fragment"
''',
    )
    write(work / "bad.sh", "#!/bin/sh\nprintf '%s\\n' value # narration\n")
    violations, _ = checker.scan_paths([work], reference)
    require(reasons(violations) == ["shell comments are not allowed"], "shell comment policy failed")


def test_toml_comments_and_strings(work: Path, reference: Path) -> None:
    write(
        work / "clean.toml",
        '''basic = "# data"
literal = '# data'
multiline = """# data"""
''',
    )
    write(work / "bad.toml", 'value = "data" # narration\n')
    write(
        work / "Cargo.lock",
        "# This file is automatically @generated by Cargo.\n"
        "# It is not intended for manual editing.\n"
        "version = 4\n",
    )
    violations, _ = checker.scan_paths([work], reference)
    require(
        reasons(violations).count("TOML comments are not allowed") == 1,
        "TOML comment policy failed",
    )


def test_javascript_comments_and_literals(work: Path, reference: Path) -> None:
    write(
        work / "clean.js",
        '''const url = "https://example.com/path//data";
const block = '/* data */';
const template = `https://example.com//data /* data */ ${url}`;
const regex = /https?:\\/\\//;
''',
    )
    write(work / "clean.mjs", "export const url = 'https://example.com/path';\n")
    write(
        work / "bad.js",
        '''const one = 1; // narration
const two = 2; /* narration */
const nested = `${one /* narration */}`;
''',
    )
    write(work / "bad.mjs", "export const value = 1; // narration\n")
    violations, _ = checker.scan_paths([work], reference)
    found = reasons(violations)
    require(found.count("JavaScript line comments are not allowed") == 2, "JavaScript line comment passed")
    require(found.count("JavaScript block comments are not allowed") == 2, "JavaScript block comment passed")


def test_css_block_comments_and_strings(work: Path, reference: Path) -> None:
    write(
        work / "clean.css",
        '.clean { content: "/* data */"; background: url("https://example.com/image.png"); }\n',
    )
    write(work / "bad.css", ".bad { color: red; /* narration */ }\n")
    violations, _ = checker.scan_paths([work], reference)
    require(reasons(violations) == ["CSS block comments are not allowed"], "CSS block comment passed")


def git(repository: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", "-C", str(repository), *arguments],
        check=True,
        capture_output=True,
    )


def test_repository_file_selection_and_data_exemptions(work: Path, reference: Path) -> None:
    repository = work / "repository"
    repository.mkdir(parents=True)
    git(repository, "init", "--quiet")
    write(repository / ".gitignore", "ignored.rs\n")
    write(repository / "tracked.rs", "fn tracked() {}\n")
    write(repository / "ignored.rs", "// narration\n")
    write(repository / "new.rs", "fn new_source() {}\n")
    write(repository / "Cargo.lock", "version = 4\n")
    write(repository / "LICENSE", "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n")
    write(repository / "data.json", '{"text":"// data"}\n')
    write(repository / "notes.txt", "# plain data\n")
    write(repository / "font.ttf", "binary data")
    write(repository / "image.png", "binary data")
    write(repository / "fixture.dex", "binary data")
    git(
        repository,
        "add",
        ".gitignore",
        "tracked.rs",
        "Cargo.lock",
        "data.json",
        "notes.txt",
        "font.ttf",
        "image.png",
        "fixture.dex",
    )

    files = {path.relative_to(repository) for path in checker.collect_repository_files(repository)}
    require(Path("tracked.rs") in files, "tracked source file was not selected")
    require(Path("new.rs") in files, "untracked source file was not selected")
    require(Path("ignored.rs") not in files, "ignored source file was selected")
    violations, _ = checker.scan_repository(repository, reference)
    require(not violations, f"repository data exemptions failed: {reasons(violations)}")

    write(repository / "unknown.kt", "val value = 1\n")
    git(repository, "add", "unknown.kt")
    violations, _ = checker.scan_repository(repository, reference)
    require(
        "comment policy checker does not support this source/test file type" in reasons(violations),
        "unknown tracked code type passed",
    )


def test_unknown_file_type(work: Path, reference: Path) -> None:
    write(work / "src" / "unchecked.kt", "// hidden\n")
    violations, _ = checker.scan_paths([work / "src"], reference)
    require(
        "comment policy checker does not support this source/test file type" in reasons(violations),
        "unknown source file type passed without inspection",
    )


def test_c_sources_and_shared_objects(work: Path, reference: Path) -> None:
    write(
        work / "src" / "clean.c",
        '// cite: frameworks-base Framework.java#method\nconst char *url = "http://example.com";\nchar quote = \'"\';\n',
    )
    write(work / "src" / "clean.h", "#pragma once\n")
    (work / "src" / "libfixture.so").write_bytes(b"\x7fELF// binary")
    violations, _ = checker.scan_paths([work / "src"], reference)
    require(not violations, f"clean C inputs failed: {reasons(violations)}")
    write(work / "src" / "bad.c", "/* block */\nint value; // narration\n")
    violations, _ = checker.scan_paths([work / "src"], reference)
    require(
        reasons(violations).count("block comments are not allowed") == 1
        and "comments must be standalone citation lines" in reasons(violations),
        "C comment violations were not reported",
    )


def test_failure_guidance() -> None:
    violation = checker.Violation(Path("src/example.rs"), 3, 1, "bad comment", "// bad")
    output = io.StringIO()
    checker.write_failure_report([violation], output)
    report = output.getvalue()
    require("// cite: <repo> <FileName>#<anchor>" in report, "allowed form is missing")
    require("TODO / FIXME / NOTE / HACK / XXX" in report, "banned forms are missing")
    require("commented-out code" in report, "commented-out code guidance is missing")
    require("line-number anchors" in report, "line-number guidance is missing")


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        reference = root / "android-ref"
        fake_reference(reference)
        test_valid_sources(root / "valid", reference)
        test_comment_forms(root / "comments", reference)
        test_citation_grammar(root / "grammar", reference)
        test_java_citation_spelling(root / "java-spelling", reference)
        test_repeated_source_block(root / "blocks", reference)
        test_reference_resolution(root / "sources", reference)
        test_exact_anchor_resolution(root / "exact-anchor", reference)
        test_python_comments_and_docstrings(root / "python", reference)
        test_shell_comments_and_shebangs(root / "shell", reference)
        test_toml_comments_and_strings(root / "toml", reference)
        test_javascript_comments_and_literals(root / "javascript", reference)
        test_css_block_comments_and_strings(root / "css", reference)
        test_repository_file_selection_and_data_exemptions(root / "repository", reference)
        test_unknown_file_type(root / "unknown", reference)
        test_c_sources_and_shared_objects(root / "c-sources", reference)
    test_failure_guidance()
    print("comment citation checker self-test passed")


if __name__ == "__main__":
    main()
