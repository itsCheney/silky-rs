// Small C interface over pl_mpeg for the movie player in movie.rs, so the
// Rust side needs no struct layouts.
#include <stddef.h>
#include <stdio.h>
#define PL_MPEG_IMPLEMENTATION
#include "pl_mpeg.h"

plm_t *silky_plm_open(uint8_t *bytes, size_t length, int audio) {
    plm_t *p = plm_create_with_memory(bytes, length, 0);
    if (!p) {
        return NULL;
    }
    plm_set_video_enabled(p, !audio);
    plm_set_audio_enabled(p, audio);
    plm_set_loop(p, 0);
    return p;
}

void silky_plm_close(plm_t *p) { plm_destroy(p); }

int silky_plm_width(plm_t *p) { return plm_get_width(p); }
int silky_plm_height(plm_t *p) { return plm_get_height(p); }
double silky_plm_framerate(plm_t *p) { return plm_get_framerate(p); }
int silky_plm_samplerate(plm_t *p) { return plm_get_samplerate(p); }

// Decodes the next video frame into `rgba` (width * 4 per row). Returns 0
// at the end of the stream.
int silky_plm_next_frame(plm_t *p, uint8_t *rgba, double *time) {
    plm_frame_t *f = plm_decode_video(p);
    if (!f) {
        return 0;
    }
    plm_frame_to_rgba(f, rgba, f->width * 4);
    *time = f->time;
    return 1;
}

// Decodes the next audio frame into `out` (interleaved stereo, room for
// PLM_AUDIO_SAMPLES_PER_FRAME * 2 floats). Returns the number of samples
// per channel, 0 at the end.
int silky_plm_next_audio(plm_t *p, float *out) {
    plm_samples_t *s = plm_decode_audio(p);
    if (!s) {
        return 0;
    }
    memcpy(out, s->interleaved, sizeof(float) * s->count * 2);
    return (int)s->count;
}
