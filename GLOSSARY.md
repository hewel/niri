# Personal niri

Terms for the personal niri desktop, its rendering, and choosing the build used at login.

## Language

**Accepted build**:
A personal niri build confirmed by the user within a stated validation scope. Acceptance of corner appearance does not imply acceptance of HDR output or screen capture.
An explicit user instruction can also approve a build for daily trial before human validation; its record must distinguish authorization from observed success.

**Running build**:
The niri build used by the current desktop session. It can differ from the build selected for the next login.

**Daily build**:
The accepted personal niri build selected for the next personal desktop login. Selecting a daily build does not replace an already running desktop session.

**Rollback build**:
The preceding accepted personal build selected for daily use. The system-provided niri build is a separate fallback, with its own configuration compatibility requirements.

**Daily deployment**:
The process of making an accepted build available as the daily build while retaining a rollback build.

**Xray background**:
The background sampled by a window effect independently of the foreground windows beneath it. Background content hidden from a capture must remain hidden when it is sampled by that capture's effects or window animations.

**Capture target**:
The intended audience of a rendered scene: the live output, a screencast, or another screen capture. Its visibility rules are independent of the image's color space and storage format.

**SDR capture**:
A captured image in SDR color space, including conversion of HDR content using the source output's reference luminance.

**Window unmap**:
A mapped window leaving the desktop. Removing its buffer retains the toplevel for a fresh configuration and later mapping; destroying the toplevel does not.
