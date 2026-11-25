// Small C interface over FFmpeg (libavformat and libavcodec only) for the
// movie player in movie_ff.rs, so the Rust side needs no struct layouts.
// One handle decodes one stream: the pictures or the sound of a movie held
// in memory.
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/mem.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    const uint8_t *data;
    size_t size;
    size_t pos;
} Mem;

typedef struct {
    Mem mem;
    AVFormatContext *fmt;
    AVCodecContext *dec;
    AVPacket *pkt;
    AVFrame *frame;
    int stream;
    int video;
    int flushed;
    int64_t count;
    float *pcm;
    size_t pcm_cap;
} Ff;

static int mem_read(void *opaque, uint8_t *buf, int size) {
    Mem *m = opaque;
    size_t left = m->size - m->pos;
    if (left == 0) {
        return AVERROR_EOF;
    }
    size_t n = left < (size_t)size ? left : (size_t)size;
    memcpy(buf, m->data + m->pos, n);
    m->pos += n;
    return (int)n;
}

static int64_t mem_seek(void *opaque, int64_t offset, int whence) {
    Mem *m = opaque;
    if (whence == AVSEEK_SIZE) {
        return (int64_t)m->size;
    }
    int64_t target;
    switch (whence & ~AVSEEK_FORCE) {
    case SEEK_SET: target = offset; break;
    case SEEK_CUR: target = (int64_t)m->pos + offset; break;
    case SEEK_END: target = (int64_t)m->size + offset; break;
    default: return -1;
    }
    if (target < 0 || target > (int64_t)m->size) {
        return -1;
    }
    m->pos = (size_t)target;
    return target;
}

void silky_ff_close(Ff *f) {
    if (!f) {
        return;
    }
    avcodec_free_context(&f->dec);
    if (f->fmt) {
        // The AVIOContext is ours to free; its buffer may have been replaced.
        if (f->fmt->pb) {
            av_freep(&f->fmt->pb->buffer);
            avio_context_free(&f->fmt->pb);
        }
        avformat_close_input(&f->fmt);
    }
    av_packet_free(&f->pkt);
    av_frame_free(&f->frame);
    free(f->pcm);
    free(f);
}

// Opens the first picture stream (`video` 1) or sound stream (0) of the
// bytes, which stay owned by the caller and must outlive the handle.
Ff *silky_ff_open(const uint8_t *bytes, size_t length, int video) {
    Ff *f = calloc(1, sizeof(Ff));
    if (!f) {
        return NULL;
    }
    f->mem = (Mem){bytes, length, 0};
    f->video = video;
    f->fmt = avformat_alloc_context();
    unsigned char *buf = av_malloc(1 << 16);
    AVIOContext *io = buf ? avio_alloc_context(buf, 1 << 16, 0, &f->mem, mem_read, NULL, mem_seek) : NULL;
    if (!f->fmt || !io) {
        av_free(buf);
        silky_ff_close(f);
        return NULL;
    }
    f->fmt->pb = io;
    f->fmt->flags |= AVFMT_FLAG_CUSTOM_IO;
    if (avformat_open_input(&f->fmt, NULL, NULL, NULL) < 0) {
        // avformat_open_input frees the context on failure, leaving fmt NULL.
        av_freep(&io->buffer);
        avio_context_free(&io);
        silky_ff_close(f);
        return NULL;
    }
    if (avformat_find_stream_info(f->fmt, NULL) < 0) {
        silky_ff_close(f);
        return NULL;
    }
    enum AVMediaType want = video ? AVMEDIA_TYPE_VIDEO : AVMEDIA_TYPE_AUDIO;
    f->stream = av_find_best_stream(f->fmt, want, -1, -1, NULL, 0);
    if (f->stream < 0) {
        silky_ff_close(f);
        return NULL;
    }
    for (unsigned i = 0; i < f->fmt->nb_streams; i++) {
        if ((int)i != f->stream) {
            f->fmt->streams[i]->discard = AVDISCARD_ALL;
        }
    }
    AVStream *st = f->fmt->streams[f->stream];
    const AVCodec *codec = avcodec_find_decoder(st->codecpar->codec_id);
    f->dec = codec ? avcodec_alloc_context3(codec) : NULL;
    if (!f->dec || avcodec_parameters_to_context(f->dec, st->codecpar) < 0 ||
        avcodec_open2(f->dec, codec, NULL) < 0) {
        silky_ff_close(f);
        return NULL;
    }
    f->pkt = av_packet_alloc();
    f->frame = av_frame_alloc();
    if (!f->pkt || !f->frame) {
        silky_ff_close(f);
        return NULL;
    }
    return f;
}

int silky_ff_width(Ff *f) { return f->dec->width; }
int silky_ff_height(Ff *f) { return f->dec->height; }
int silky_ff_samplerate(Ff *f) { return f->dec->sample_rate; }
int silky_ff_channels(Ff *f) { return f->dec->ch_layout.nb_channels; }

double silky_ff_framerate(Ff *f) {
    AVStream *st = f->fmt->streams[f->stream];
    AVRational r = st->avg_frame_rate.num ? st->avg_frame_rate : st->r_frame_rate;
    return r.den ? (double)r.num / r.den : 0.0;
}

// Next decoded frame of the stream, or 0 at the end.
static int next_decoded(Ff *f) {
    for (;;) {
        int r = avcodec_receive_frame(f->dec, f->frame);
        if (r == 0) {
            return 1;
        }
        if (r != AVERROR(EAGAIN)) {
            return 0;
        }
        for (;;) {
            r = av_read_frame(f->fmt, f->pkt);
            if (r < 0) {
                if (f->flushed) {
                    return 0;
                }
                f->flushed = 1;
                avcodec_send_packet(f->dec, NULL);
                break;
            }
            if (f->pkt->stream_index == f->stream) {
                avcodec_send_packet(f->dec, f->pkt);
                av_packet_unref(f->pkt);
                break;
            }
            av_packet_unref(f->pkt);
        }
    }
}

static uint8_t clamp(int n) { return n < 0 ? 0 : n > 255 ? 255 : (uint8_t)n; }

// YUV 4:2:0 to RGBA, BT.601 limited range, with the integers pl_mpeg uses.
// Alpha is set to 255. `stride` of the output is width * 4.
void silky_ff_yuv420_to_rgba(const uint8_t *y, int ys, const uint8_t *cb, int cbs,
                             const uint8_t *cr, int crs, int width, int height,
                             uint8_t *rgba) {
    for (int row = 0; row < height; row++) {
        const uint8_t *yp = y + (size_t)row * ys;
        const uint8_t *cbp = cb + (size_t)(row >> 1) * cbs;
        const uint8_t *crp = cr + (size_t)(row >> 1) * crs;
        uint8_t *d = rgba + (size_t)row * width * 4;
        for (int col = 0; col < width; col++) {
            int u = cbp[col >> 1] - 128;
            int v = crp[col >> 1] - 128;
            int r = (v * 104597) >> 16;
            int g = (u * 25674 + v * 53278) >> 16;
            int b = (u * 132201) >> 16;
            int l = ((yp[col] - 16) * 76309) >> 16;
            d[0] = clamp(l + r);
            d[1] = clamp(l - g);
            d[2] = clamp(l + b);
            d[3] = 255;
            d += 4;
        }
    }
}

// Decodes the next picture into `rgba` (width * height * 4 bytes) and gives
// its time in seconds. Returns 0 at the end of the stream.
int silky_ff_next_frame(Ff *f, uint8_t *rgba, double *time) {
    if (!f->video || !next_decoded(f)) {
        return 0;
    }
    AVFrame *fr = f->frame;
    AVStream *st = f->fmt->streams[f->stream];
    int64_t pts = fr->best_effort_timestamp;
    double fps = silky_ff_framerate(f);
    if (pts == AV_NOPTS_VALUE) {
        *time = fps > 0 ? (double)f->count / fps : 0.0;
    } else {
        *time = (double)pts * av_q2d(st->time_base);
    }
    f->count++;
    if (fr->format == AV_PIX_FMT_YUV420P || fr->format == AV_PIX_FMT_YUVJ420P) {
        silky_ff_yuv420_to_rgba(fr->data[0], fr->linesize[0], fr->data[1], fr->linesize[1],
                                fr->data[2], fr->linesize[2], fr->width, fr->height, rgba);
    } else {
        memset(rgba, 0, (size_t)fr->width * fr->height * 4);
    }
    return 1;
}

// Decodes the next block of sound and hands out its first two channels as
// interleaved floats (a mono track is doubled). `*out` stays valid until
// the next call. Returns the samples per channel, 0 at the end.
int silky_ff_next_audio(Ff *f, const float **out) {
    if (f->video) {
        return 0;
    }
    for (;;) {
        if (!next_decoded(f)) {
            return 0;
        }
        AVFrame *fr = f->frame;
        int n = fr->nb_samples;
        int ch = fr->ch_layout.nb_channels;
        if (n <= 0 || ch <= 0) {
            continue;
        }
        if (f->pcm_cap < (size_t)n * 2) {
            float *p = realloc(f->pcm, (size_t)n * 2 * sizeof(float));
            if (!p) {
                return 0;
            }
            f->pcm = p;
            f->pcm_cap = (size_t)n * 2;
        }
        int planar = av_sample_fmt_is_planar(fr->format);
        for (int i = 0; i < n; i++) {
            for (int c = 0; c < 2; c++) {
                int src = c < ch ? c : 0;
                float s;
                switch (av_get_packed_sample_fmt(fr->format)) {
                case AV_SAMPLE_FMT_FLT:
                    s = planar ? ((const float *)fr->extended_data[src])[i]
                               : ((const float *)fr->extended_data[0])[(size_t)i * ch + src];
                    break;
                case AV_SAMPLE_FMT_S16:
                    s = (planar ? ((const int16_t *)fr->extended_data[src])[i]
                                : ((const int16_t *)fr->extended_data[0])[(size_t)i * ch + src]) /
                        32768.0f;
                    break;
                case AV_SAMPLE_FMT_S32:
                    s = (planar ? ((const int32_t *)fr->extended_data[src])[i]
                                : ((const int32_t *)fr->extended_data[0])[(size_t)i * ch + src]) /
                        2147483648.0f;
                    break;
                default:
                    s = 0.0f;
                }
                f->pcm[(size_t)i * 2 + c] = s;
            }
        }
        *out = f->pcm;
        return n;
    }
}
