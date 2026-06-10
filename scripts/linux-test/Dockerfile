# Minimal Linux desktop runtime to test Mainbound's AppImage against the
# REAL WebKitGTK engine (the same one that OOM-kills on the user's machine),
# with the GUI forwarded to the host Mac via XQuartz.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Tauri's runtime deps on Linux: WebKitGTK 4.1 + GTK + the libs the
# webview needs, plus libsecret for the keyring and a few X11 utils.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-0 \
    libgtk-3-0 \
    libayatana-appindicator3-1 \
    librsvg2-2 \
    libsecret-1-0 \
    libnotify4 \
    libgl1-mesa-dri \
    libgl1-mesa-glx \
    mesa-utils \
    git \
    ca-certificates \
    procps \
    fuse \
    file \
    x11-apps \
    && rm -rf /var/lib/apt/lists/*

# WebKitGTK env: same workarounds the app sets, plus software GL since the
# container has no real GPU (forwarded X11 can't use host Metal).
ENV WEBKIT_DISABLE_DMABUF_RENDERER=1
ENV WEBKIT_DISABLE_COMPOSITING_MODE=1
ENV LIBGL_ALWAYS_SOFTWARE=1
ENV GDK_BACKEND=x11

WORKDIR /app

# Entrypoint extracts the mounted AppImage (no FUSE over X11) and runs it
COPY run-appimage.sh /usr/local/bin/run-appimage.sh
RUN chmod +x /usr/local/bin/run-appimage.sh

ENTRYPOINT ["/usr/local/bin/run-appimage.sh"]
