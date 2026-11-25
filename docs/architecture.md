# Architecture

[Back to README](../README.md)

Runtime boundary
----------------

winedroid is not a VM, emulator, container wrapper, or hidden Android install.
The runtime must be owned by winedroid.

Out of scope:

- hardware or system virtual machines
- full Android system images as the source of truth
- container-first Android systems
- adb-installing into a separate Android OS
- depending on Android `system_server` as the runtime owner
- host-side shortcuts that bypass APK/framework behavior

In scope:

- parsing APKs, manifests, resources, DEX files, and native libraries
- executing DEX through ART or a winedroid-owned DEX runtime strategy
- Android framework class compatibility
- winedroid-owned package, activity, resource, lifecycle, and service models
- Android ELF loading with a Bionic-compatible runtime
- native CPU translation when APK ABI and host CPU differ
- host-native windows, input, storage, networking, clipboard, and permissions

The distinction matters: ART is a managed bytecode runtime, not a hardware or
system VM. Reusing ART is acceptable only if winedroid owns the surrounding
process model and services instead of booting Android.

Architecture laws
-----------------

These rules are intended to keep the codebase from collapsing into catch-all
modules.

- One module owns one responsibility.
- Refactors must move existing behavior first; behavior changes happen in later
  patches.
- Do not replace implemented paths with stubs during refactors.
- Unsupported Android behavior must be explicit. Best-effort execution logs
  missing calls and substitutes diagnostic return values at the call boundary;
  WINEDROID_STRICT=1 rejects them. Neither mode treats substitutes as implemented
  behavior. Runtime errors and guest exceptions still propagate.
- No app-specific rendering hacks. Compatibility work must become a generic
  Android/runtime primitive.
- Rendering reads UI state and draws pixels. It must not mutate runtime state.
- Host integration must go through narrow explicit bridges.
- Launch/session code wires components together. It must not become a runtime,
  renderer, window manager, or Android framework implementation.
- Avoid framework overengineering: no trait soup, plugin systems, or dynamic
  dispatch layers unless a concrete compatibility need forces them.
- Do not optimize runtime performance until behavior and module boundaries are
  stable.

Layer ownership
---------------

Target dependency direction:

```text
APK -> Dex/ClassLoader -> Runtime -> Android framework -> Services/UI -> Renderer/Host
```

Layer responsibilities:

- `apk`: APK archive access and package file inspection.
- `manifest`: Android manifest parsing and launchable component metadata.
- `resource_table`: compiled Android resource table parsing and lookup.
- `binary_xml`: Android binary XML decoding.
- `dex`: DEX file parsing, class/method/field metadata, and bytecode data.
- `dex_interpreter`: DEX instruction execution mechanics only.
- `runtime`: object state, heap, arrays, fields, class loading, method dispatch,
  and builtin routing.
- `android`: Android Java framework classes grouped by package/domain.
- `services`: emulated Android system services such as activity manager,
  package manager, resource manager, window manager, storage, input, and
  clipboard.
- `ui`: Android UI state, view trees, layout, hit testing, overlays, active
  surfaces, and input routing.
- `render`: pixels from UI state. Skia belongs here.
- `host`: OS integration: windows, event loops, browser opening, platform input,
  files, clipboard, notifications, and platform-specific glue.
- `native`: ABI selection, ELF loading, JNI, linker behavior, Bionic-compatible
  runtime, and CPU translation hooks.
- `unwind`: the guest unwinder: FDE lookup, the DWARF CFI interpreter, and the
  `_Unwind_*` phase machine.
- `launch`: app startup/session setup only.

Runtime contract
----------------

The target runtime model is Android-shaped. If code cannot preserve this model,
it should report a clear missing primitive. The call boundary may continue in
best-effort mode; strict validation must reject that missing behavior.

- Install produces reusable package artifacts: manifest identity, launch
  component, APK source metadata, DEX metadata, resources.arsc summary, resource
  file index, selected ABI, native inventory, and installed paths.
- Launch consumes installed package artifacts and starts an app process. It
  should not keep reparsing install-time package metadata as the source of
  truth.
- The VM owns class loading, loaded class records, dex caches, resolved
  method/field lookup, string/type/proto lookup, object storage, static fields,
  arrays, and method dispatch.
- The main thread owns deterministic task dispatch. ActivityThread lifecycle
  work, Handler posts, Choreographer callbacks, and ViewRoot traversal must flow
  through the main-thread queue instead of direct launch-time shortcuts.
- Activity lifecycle is scheduled as ActivityThread-style work. Direct calls are
  acceptable only inside the lifecycle task handler that owns the Android order.
- ViewRoot traversal is request-driven. `requestLayout` schedules measure,
  layout, and draw; `invalidate` schedules draw; no request means the last
  committed frame remains current.
- Rendering is retained-frame based. Framework/app code records runtime canvas
  frames or display-list-equivalent state, and the host presents the latest
  committed frame.
- Compose is normal app bytecode running on the VM plus Android framework
  primitives. Do not add Compose-specific rendering shortcuts or app-specific
  branches to make one APK look correct.
- Native execution is either a real JNI/ELF/Bionic strategy or explicitly
  unsupported until native code is reached. Do not silently treat native code as
  successful.

Current refactor pressure
-------------------------

The project is still early, but several files have grown beyond their intended
responsibility. Before adding deeper compatibility features, prefer extracting
clear boundaries from existing working code.

Primary extraction targets:

- `launch`: keep as orchestration; move worker plumbing and host projection out.
- `dex_run.rs`: split runtime state, builtins, dispatch, and Android framework
  behavior.
- `dex_run/view.rs`: split by Android concepts such as View, ViewGroup,
  TextView, Canvas, Dialog, Preference, and Compose/custom View support.
- `layout_render.rs`: split resource/layout loading, measurement, and render
  projection.
- `host_window.rs`: keep focused on host window and input integration.

Compatibility workflow
----------------------

New app support is trace-driven, generic, and grounded in the real Android
sources. The canonical workflow, coding rules, source-of-truth policy, and
testing harness live in [AGENTS.md](../AGENTS.md).

Unwinding and dynamic loading
-----------------------------

A guest C++ library brings its own exception machinery. `libc++_shared.so`
carries a hidden copy of libunwind and libc++abi, imports no `_Unwind_*`
symbol, and finds the module that owns a program counter through
`dl_iterate_phdr` and that module's `PT_GNU_EH_FRAME` segment. That path is the
primary one and needs only the shim's `dl_iterate_phdr`, which reports
`dlpi_addr`, `dlpi_name`, the mapped phdr array, `dlpi_phnum` as a 16 bit
field, `dlpi_adds`, `dlpi_subs`, `dlpi_tls_modid` and `dlpi_tls_data`.
`dlpi_tls_data` is the block of the calling thread and stays null until that
thread materializes it, matching Bionic's `should_alloc=false` read.

The shim also implements the arm64 `_Unwind_*` surface that Bionic's libc.so
exports, for C libraries built with `-fexceptions` that link no C++ runtime.
That unwinder interprets DWARF CFI itself over guest `.eh_frame` records. It
runs on guest memory only: the four entries that capture or install a full
register set are raw assembly (`src/native/unwind/entry.rs`), FDEs come from
`__register_frame` blocks or from each loaded module's `.eh_frame_hdr`, and the
CFI interpreter restores registers by carrying each frame's live values forward
and overwriting only the registers a row gives a rule for.

Both unwinders share no context. A context created by the shim carries a magic
word, and an accessor handed anything else refuses with
`next missing primitive: cross-unwinder _Unwind_Context` instead of
interpreting a foreign object. A personality that lives in a module which
carries its own unwinder is refused with
`next missing primitive: unwinding through a personality that uses its own
unwinder`, because it would read the shim's context as its own cursor type.
The mixed case, a C library that throws with a personality resolving to
libc.so and then passes through a `libc++_shared` frame, is therefore refused
rather than silently mis-unwound.

Known limits of this layer, all explicit refusals rather than fallbacks:

- `dl_iterate_phdr` lists the 13 system sonames with `dlpi_phdr = 0` and
  `dlpi_phnum = 0`. They have no mapped guest phdr table, and a phdr reader
  treats an empty array as inert. There is no entry for a guest executable or
  the vdso.
- The iteration snapshots the module list, so a module loaded inside the
  callback is not visited in that pass.
- A walk ends at the boundary of a host shim frame, because stub code has no
  CFI. The guest's own unwinder ends there too.
- `%Lf`, `%Lg`, `%Le`, `%La` and `%ls` in the printf engine remain explicit
  missing primitive errors, so `std::cout << (long double)` and
  `std::cout << (wchar_t const*)` fail with a named primitive.
- `__emutls_get_address` is not exported, matching `bionic/libc/libc.map.txt`.
  Resolution depends on a guest library that defines it, as on Android.

Optimization policy
-------------------

Do not optimize first. Correctness and boundaries matter more right now.
Allowed now:

- smaller modules
- simpler ownership
- clearer error paths
- fewer fallbacks
- lower coupling
- less duplicated projection logic

Deferred until the model is stable:

- runtime performance tuning
- renderer caching
- interpreter hot-path optimization
- speculative native execution shortcuts
- broad fallback systems

