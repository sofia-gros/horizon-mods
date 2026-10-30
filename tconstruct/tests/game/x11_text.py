"""Type through the real X11 keyboard into a selected test game's window."""
import ctypes


def type_text(game, text):
    x = ctypes.CDLL('libX11.so.6')
    xt = ctypes.CDLL('libXtst.so.6')
    display_t, window_t = ctypes.c_void_p, ctypes.c_ulong
    x.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x.XOpenDisplay.restype = display_t
    x.XDefaultRootWindow.argtypes = [display_t]
    x.XDefaultRootWindow.restype = window_t
    x.XQueryTree.argtypes = [display_t, window_t, ctypes.POINTER(window_t), ctypes.POINTER(window_t), ctypes.POINTER(ctypes.POINTER(window_t)), ctypes.POINTER(ctypes.c_uint)]
    x.XInternAtom.argtypes = [display_t, ctypes.c_char_p, ctypes.c_int]
    x.XInternAtom.restype = window_t
    x.XGetWindowProperty.argtypes = [display_t, window_t, window_t, ctypes.c_long, ctypes.c_long, ctypes.c_int, window_t, ctypes.POINTER(window_t), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte))]
    x.XFree.argtypes = [ctypes.c_void_p]
    x.XSetInputFocus.argtypes = [display_t, window_t, ctypes.c_int, ctypes.c_ulong]
    x.XStringToKeysym.argtypes = [ctypes.c_char_p]
    x.XStringToKeysym.restype = window_t
    x.XKeysymToKeycode.argtypes = [display_t, window_t]
    x.XKeysymToKeycode.restype = ctypes.c_ubyte
    x.XSync.argtypes = [display_t, ctypes.c_int]
    x.XCloseDisplay.argtypes = [display_t]
    xt.XTestFakeKeyEvent.argtypes = [display_t, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    display = x.XOpenDisplay(game.env.get('DISPLAY', '').encode())
    if not display:
        raise RuntimeError('Cannot open the game test display')
    try:
        atom = x.XInternAtom(display, b'_NET_WM_PID', False)
        def find(window):
            actual, fmt, count, extra = window_t(), ctypes.c_int(), ctypes.c_ulong(), ctypes.c_ulong()
            data = ctypes.POINTER(ctypes.c_ubyte)()
            if x.XGetWindowProperty(display, window, atom, 0, 1, False, 6, ctypes.byref(actual), ctypes.byref(fmt), ctypes.byref(count), ctypes.byref(extra), ctypes.byref(data)) == 0 and data:
                try:
                    if fmt.value == 32 and count.value and ctypes.cast(data, ctypes.POINTER(ctypes.c_ulong))[0] == game.proc.pid:
                        return window
                finally:
                    x.XFree(data)
            root, parent, children, count = window_t(), window_t(), ctypes.POINTER(window_t)(), ctypes.c_uint()
            if x.XQueryTree(display, window, ctypes.byref(root), ctypes.byref(parent), ctypes.byref(children), ctypes.byref(count)):
                try:
                    for child in children[:count.value]:
                        found = find(child)
                        if found: return found
                finally:
                    if children: x.XFree(children)
            return None
        window = find(x.XDefaultRootWindow(display))
        if not window:
            raise RuntimeError(f'Cannot find X11 window for test game pid {game.proc.pid}')
        x.XSetInputFocus(display, window, 1, 0)
        for char in text:
            symbol = b'space' if char == ' ' else char.encode('ascii')
            code = x.XKeysymToKeycode(display, x.XStringToKeysym(symbol))
            if not code: raise RuntimeError(f'No X11 key for {char!r}')
            xt.XTestFakeKeyEvent(display, code, True, 0)
            xt.XTestFakeKeyEvent(display, code, False, 0)
        x.XSync(display, False)
    finally:
        x.XCloseDisplay(display)
