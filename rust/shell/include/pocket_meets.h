/* Pocket Meets shell C ABI.
 *
 * Build the static library on a Mac:
 *   cargo build --release --target aarch64-apple-ios \
 *     --manifest-path rust/shell/Cargo.toml
 * The archive is
 *   rust/shell/target/aarch64-apple-ios/release/libpocket_meets.a
 * Simulator:
 *   cargo build --release --target aarch64-apple-ios-sim \
 *     --manifest-path rust/shell/Cargo.toml
 *
 * Link with -lpocket_meets and the system libraries the Rust staticlib
 * needs (libSystem is enough for this crate; add resolv if the linker asks).
 * Call every function from one serial queue. Free strings with pm_string_free.
 * Mixing client version sent to the server is 1.4.3. The notes say 1.0.1.
 */
#ifndef POCKET_MEETS_H
#define POCKET_MEETS_H

#include <stdint.h>
#include <stddef.h>

#define PM_AUDIO_RATE 44100
#define PM_LCD 128
#define PM_FRAME_PIXELS 16384
#define PM_FRAME_NS 16666667u
#define PM_TICK_VISIBLE_MS 8u
#define PM_TICK_HIDDEN_MS 100u
#define PM_BLE_NAME "TMGC_meets"
#define PM_MIXING_VERSION "1.4.3"
#define PM_RELEASE "1.0.1"
#define PM_BUILD 10018
#define PM_GATT_IDLE_MS 300000
#define PM_GATT_ACTIVE_MS 120000
#define PM_GATT_GRACE_MS 2500
#define PM_GATT_COMMIT_MS 300
#define PM_NAME_POLLS 25
#define PM_NAME_POLL_MS 200
#define PM_RX_QUEUE 128
#define PM_TX_QUEUE 512
#define PM_CALL_NOTIFICATION_ID 2
#define PM_CARE_FAIL_NOTIFICATION_ID 3
#define PM_SERVICE_UUID "0000fff0-0000-1000-8000-00805f9b34fb"
#define PM_TX_UUID "0000fff1-0000-1000-8000-00805f9b34fb"
#define PM_CCCD_UUID "00002902-0000-1000-8000-00805f9b34fb"
#define PM_RX_UUID "0000fff2-0000-1000-8000-00805f9b34fb"

/* Complete so Swift can name the pointer. The fields are not a layout. */
typedef struct PmSession {
    char unused;
} PmSession;
typedef struct PmForecast {
    char unused;
} PmForecast;

typedef struct PmPumpIn {
    double now_unix;
    double dt;
    int visible;
    unsigned long long monotonic_ms;
    unsigned long long last_frame_ns;
    unsigned long long last_save_ms;
    unsigned long long last_catchup_save_ms;
    double backlog;
    int phone_interactive;
    int follow_system_sleep;
    int muted;
    unsigned long long frame_ns;
    /* 1 while Picture in Picture is watching the LCD. Hidden ticks still publish frames. */
    int present;
} PmPumpIn;

typedef struct PmPumpOut {
    int stopped;
    int step_code;
    int route;
    int take_call;
    int play_audio;
    float volume;
    int drew_frame;
    int did_save;
    int catching_up;
    int tick;
    unsigned long long next_ms;
    double backlog;
    double remain;
    double catchup_total;
    unsigned long long last_frame_ns;
    unsigned long long last_save_ms;
    unsigned long long last_catchup_save_ms;
    int pcm_count;
} PmPumpOut;

void pm_string_free(char *s);
char *pm_text(int lang, const char *key);
char *pm_localize(const char *text, int lang);
char *pm_player_name(PmSession *s);
char *pm_message_text(PmSession *s);
int32_t pm_current_is_sweets(PmSession *s);
/* pixels points to len ARGB pixels. The returned string must be freed. */
char *pm_menu_text(const uint32_t *pixels, size_t len);
int pm_dump_sram(PmSession *s, const char *path);
int pm_dump_window(PmSession *s, unsigned int start, unsigned int len, const char *path);
char *pm_mixing_version(void);
char *pm_release(void);
int pm_build(void);

int pm_rom_kind(const char *path);
char *pm_rom_name(const char *path);
int pm_has_current(const char *root);
int pm_container_ensure(const char *root);
char *pm_import_rom(const char *root, const char *path);
char *pm_library_json(const char *root);
char *pm_settings_json(const char *path);
char *pm_settings_write(const char *path, const char *json);
char *pm_plan_json(const char *path);
int pm_plan_store(const char *path, const char *json);

PmSession *pm_open(const char *rom, const char *saves);
PmSession *pm_open_care(const char *rom, const char *saves, int paused);
void pm_close(PmSession *s);
char *pm_name(PmSession *s);
char *pm_error(PmSession *s);
char *pm_notice(PmSession *s);
double pm_seconds(PmSession *s);
int pm_asleep(PmSession *s);
int pm_step(PmSession *s, double dt);
int pm_key(PmSession *s, int index, int down);
void pm_release_keys(PmSession *s);
void pm_set_shelf_open(PmSession *s, int open);
void pm_display_awake(PmSession *s, int on);
void pm_set_fast_keys(PmSession *s, int on);
int pm_display_hint(int visible, int active, int paused, int follow, int interactive, int present);
void pm_frame(PmSession *s, unsigned int *argb16384);
int pm_take_call(PmSession *s);
int pm_save(PmSession *s);
int pm_delete_current_save(PmSession *s);
char *pm_import_save(PmSession *s, const char *path);
char *pm_export_save(PmSession *s, const char *path);

void pm_link_enable(PmSession *s, int on);
int pm_link_state(PmSession *s);
int pm_link_kind(PmSession *s);
int pm_link_web_meets(PmSession *s);
int pm_link_recovered(PmSession *s);
void pm_link_connect(PmSession *s);
void pm_link_cancel(PmSession *s);
int pm_link_write(PmSession *s, const unsigned char *bytes, int len);
int pm_link_read(PmSession *s, unsigned char *out, int cap);

int pm_ir_write(PmSession *s, const unsigned char *bytes, int len);
int pm_ir_read(PmSession *s, unsigned char *out, int cap);

double pm_care_sync(PmSession *s, double now_unix, double budget);
double pm_care_at(PmSession *s);
void pm_care_pause(PmSession *s, int paused, double now_unix);
void pm_care_adopt(PmSession *s);
void pm_advance_clock(PmSession *s, double seconds);
int pm_pump(PmSession *s, const PmPumpIn *in, PmPumpOut *out, short *pcm, int pcm_cap);

char *pm_control(PmSession *s, const char *json);

PmForecast *pm_forecast_create(PmSession *s, double not_before);
PmForecast *pm_forecast_open(const char *rom, const char *cursor, double not_before);
int pm_forecast_run(PmForecast *f, int budget);
double pm_forecast_at(PmForecast *f);
double pm_forecast_event(PmForecast *f);
int pm_forecast_save(PmForecast *f, const char *path);
void pm_forecast_close(PmForecast *f);
double pm_forecast_seed(double now_unix, long long last_call_ms);
char *pm_forecast_decide(int code, double event_unix, int visible);
int pm_forecast_retain(const char *dir, long long epoch, const char *keep_json);

int pm_zone(float x, float y, float w, float h, int screenshots);
float pm_dot_strength(float pixel_points);
int pm_guide_visible(int appearance, int ready, int focused, int shelf, int guide, int shots, int seen);
int pm_exit_allowed(int ready, int shelf, int guide);
char *pm_png_name(int y, unsigned mo, unsigned d, unsigned h, unsigned mi, unsigned s, unsigned millis, const char *hex8);
char *pm_export_name(int y, unsigned mo, unsigned d, unsigned h, unsigned mi, unsigned s);

char *pm_fake_device_json(void);
char *pm_mixing_url(void);
char *pm_support_url(void);
char *pm_login_form(void);
char *pm_notify_hex(const unsigned char *bytes, int len);
int pm_gatt_rx(int owner, int subscribed, int prepared, int offset, int len, int queued);

char *pm_shelf_install(const char *root, const char *zip_path, const char *settings_path);
char *pm_shelf_send(PmSession *s, const char *tamago_path);
char *pm_shelf_balance(PmSession *s, int add, int amount);
char *pm_shelf_undo(PmSession *s);
char *pm_shelf_marry(PmSession *s, const char *json);
char *pm_custom_import(const char *root, const char *path, int kind);
void pm_set_root(PmSession *s, const char *root);
char *pm_switch_image(PmSession *s, const char *rom_path);

char *pm_care_foreground(const char *root);
int pm_care_watch(const char *root, long long now_ms);
int pm_note_heartbeat(const char *root, long long now_ms);
char *pm_alarm_exact(int allowed);

/* Last openCare / open error when the session pointer is null. */
char *pm_last_error(void);
/* JSON: detected, banner, channel, not_before_ms, last_ms. */
char *pm_call_decision(int take_call, int visible, int muted, long long now_ms, long long last_ms, long long not_before_ms);
/* 1 when a forecast alarm should post. Missed gaps and a visible toy stay quiet. */
int pm_alarm_post(long long event_ms, long long now_ms, int visible, int missed_gap);
/* Items in the installed pack for one tab. Paths are absolute. */
char *pm_shelf_list(const char *root, const char *tab, const char *language);
/* Manual enhanced-care choice. Clears the watchdog restore memory. */
char *pm_set_background_care(const char *root, int on);
/* Unified pause. Invalidates the forecast plan. The UI also calls pm_care_pause. */
int pm_set_care_paused(const char *root, int paused);

#endif
