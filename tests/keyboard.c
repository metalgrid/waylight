/* Isolated Cage test helper, not installed. Minimal Wayland virtual-keyboard
 * v1 declarations; protocol: https://github.com/atx/wtype/blob/master/protocol/virtual-keyboard-unstable-v1.xml
 * It refuses to connect outside the test's private runtime directory.
 */
#define _GNU_SOURCE
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
#include <wayland-client.h>
#include <xkbcommon/xkbcommon.h>

static const struct wl_interface keyboard_interface;
static const struct wl_interface *create_types[] = {&wl_seat_interface, &keyboard_interface};
static const struct wl_message manager_requests[] = {{"create_virtual_keyboard", "on", create_types}};
static const struct wl_interface manager_interface = {"zwp_virtual_keyboard_manager_v1", 1, 1, manager_requests, 0, NULL};
static const struct wl_message keyboard_requests[] = {
    {"keymap", "uhu", NULL}, {"key", "uuu", NULL},
    {"modifiers", "uuuu", NULL}, {"destroy", "", NULL}
};
static const struct wl_interface keyboard_interface = {"zwp_virtual_keyboard_v1", 1, 4, keyboard_requests, 0, NULL};
static struct wl_proxy *manager;
static struct wl_seat *seat;
static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    (void)data; (void)version;
    if (!strcmp(interface, manager_interface.name)) manager = wl_registry_bind(registry, name, &manager_interface, 1);
    if (!strcmp(interface, "wl_seat")) seat = wl_registry_bind(registry, name, &wl_seat_interface, 1);
}
static void removed(void *data, struct wl_registry *registry, uint32_t name) { (void)data; (void)registry; (void)name; }
int main(void) {
    const char *runtime = getenv("XDG_RUNTIME_DIR");
    const char *expected = getenv("WAYLIGHT_TEST_RUNTIME");
    assert(runtime && expected && !strcmp(runtime, expected));
    assert(strstr(runtime, "/waylight-test-") && strstr(runtime, "/runtime"));
    const char *socket = getenv("WAYLAND_DISPLAY");
    assert(socket && !strncmp(socket, "wayland-", 8) && !strchr(socket, '/'));
    struct wl_display *display = wl_display_connect(NULL); assert(display);
    struct wl_registry *registry = wl_display_get_registry(display);
    const struct wl_registry_listener listener = {global, removed};
    wl_registry_add_listener(registry, &listener, NULL);
    assert(wl_display_roundtrip(display) >= 0); assert(manager && seat);
    struct wl_proxy *keyboard = wl_proxy_marshal_flags(manager, 0, &keyboard_interface, 1, 0, seat, NULL);
    struct xkb_context *context = xkb_context_new(XKB_CONTEXT_NO_FLAGS); assert(context);
    struct xkb_rule_names names = {.layout = "us"};
    struct xkb_keymap *map = xkb_keymap_new_from_names(context, &names, XKB_KEYMAP_COMPILE_NO_FLAGS); assert(map);
    char *text = xkb_keymap_get_as_string(map, XKB_KEYMAP_FORMAT_TEXT_V1); assert(text);
    size_t size = strlen(text) + 1;
    int fd = memfd_create("test-keymap", MFD_CLOEXEC); assert(fd >= 0);
    assert(write(fd, text, size) == (ssize_t)size);
    wl_proxy_marshal_flags(keyboard, 0, NULL, 1, 0, 1u, fd, (uint32_t)size);
    assert(wl_display_roundtrip(display) >= 0); close(fd);
    puts("ready"); fflush(stdout);
    unsigned code, time = 0;
    while (scanf("%u", &code) == 1) {
        assert(code < 256);
        wl_proxy_marshal_flags(keyboard, 1, NULL, 1, 0, time++, code, 1u);
        wl_proxy_marshal_flags(keyboard, 1, NULL, 1, 0, time++, code, 0u);
        assert(wl_display_roundtrip(display) >= 0);
        puts("sent"); fflush(stdout);
    }
    wl_proxy_marshal_flags(keyboard, 3, NULL, 1, WL_MARSHAL_FLAG_DESTROY);
    wl_display_flush(display);
    free(text); xkb_keymap_unref(map); xkb_context_unref(context);
    wl_display_disconnect(display);
    return 0;
}
