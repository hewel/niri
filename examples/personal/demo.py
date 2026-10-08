#!/usr/bin/env python3
"""Static SDR layer-shell wallpaper and GTK window for an independent trial.

Launch through the custom niri's startup command so both clients inherit its sockets.
Requires the host's optional python-gobject, python-cairo and gtk-layer-shell packages.
"""
import os

os.environ["GDK_BACKEND"] = "wayland"
os.environ["NO_AT_BRIDGE"] = "1"

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("GtkLayerShell", "0.1")
from gi.repository import Gtk, GtkLayerShell, GLib

GLib.set_prgname("personal-glass-demo")


def paint(_widget, context):
    width, height = _widget.get_allocated_width(), _widget.get_allocated_height()
    context.set_source_rgb(0.12, 0.20, 0.30)
    context.paint()
    # A static SDR pattern makes refraction and color changes easy to compare.
    colors = [(0.80, 0.28, 0.22), (0.18, 0.65, 0.52), (0.30, 0.44, 0.85)]
    for row in range(0, height, 96):
        for col in range(0, width, 96):
            context.set_source_rgb(*colors[(row // 96 + col // 96) % len(colors)])
            context.rectangle(col + 8, row + 8, 80, 80)
            context.fill()
    context.set_source_rgb(0.92, 0.94, 0.98)
    context.set_line_width(3)
    context.move_to(0, height * 0.3)
    context.line_to(width, height * 0.7)
    context.stroke()


if __name__ == "__main__":
    if not os.environ.get("WAYLAND_DISPLAY") or not os.environ.get("NIRI_SOCKET"):
        raise SystemExit("Launch with: niri -c glass.kdl -- python3 examples/personal/demo.py")
    window = Gtk.Window()
    GtkLayerShell.init_for_window(window)
    GtkLayerShell.set_namespace(window, "personal-glass-wallpaper")
    GtkLayerShell.set_layer(window, GtkLayerShell.Layer.BACKGROUND)
    for edge in [GtkLayerShell.Edge.TOP, GtkLayerShell.Edge.BOTTOM,
                 GtkLayerShell.Edge.LEFT, GtkLayerShell.Edge.RIGHT]:
        GtkLayerShell.set_anchor(window, edge, True)
    GtkLayerShell.set_exclusive_zone(window, -1)
    area = Gtk.DrawingArea()
    area.connect("draw", paint)
    window.add(area)
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    panel = Gtk.Window(title="Glass drawing test")
    panel.set_name("glass-panel")
    panel.set_visual(panel.get_screen().get_rgba_visual())
    css = Gtk.CssProvider()
    css.load_from_data(b"#glass-panel { background-color: rgba(20, 30, 40, 0.35); color: white; }")
    panel.get_style_context().add_provider(css, Gtk.STYLE_PROVIDER_PRIORITY_USER)
    panel.set_default_size(600, 500)
    panel.add(Gtk.Label(label="SDR background refraction\nToggle liquid-glass off in the trial config to compare."))
    panel.show_all()
    panel.connect("destroy", Gtk.main_quit)
    Gtk.main()
