#pragma once

typedef enum {
    ENCODER_TYPE_D3D11VA_RGBA,
    ENCODER_TYPE_VULKAN_RGBA,
    ENCODER_TYPE_SOFTWARE_YUV,
} EncoderType;

typedef enum {
    FFMPEG_OPTION_VALUE_TYPE_STRING,
    FFMPEG_OPTION_VALUE_TYPE_INT,
    FFMPEG_OPTION_VALUE_TYPE_DOUBLE,
    FFMPEG_OPTION_VALUE_TYPE_RATIONAL,
    FFMPEG_OPTION_VALUE_TYPE_BINARY,
    FFMPEG_OPTION_VALUE_TYPE_DICTIONARY,
} FfmpegOptionType;

struct Rational {
    int num, den;
};

struct Binary {
    unsigned char *data;
    int length;
};

struct ImageSize {
    int width, height;
};

struct DictionaryEntry {
    const char *key;
    const char *value;
};

struct Dictionary {
    struct DictionaryEntry *entries;
    int count;
};

union FfmpegOptionValue {
    const char *string;
    long long int_value;
    double double_value;
    struct Rational rational;
    struct Binary binary;
    struct Dictionary dictionary;
};

struct FfmpegOption {
    const char *name;
    FfmpegOptionType type;
    union FfmpegOptionValue value;
};

struct EncoderAllocData {
    EncoderType type;
    const char *name;
    int resolution_width;
    int resolution_height;
    float fps;
    struct FfmpegOption *context_options;
    int context_options_count;
    struct FfmpegOption *priv_data_options;
    int priv_data_options_count;
    struct Dictionary codec_open_otpions;
    struct FfmpegOption *frame_options;
    int frame_options_count;
    struct Dictionary vendor_specific_context_options;
    struct FfmpegOption *hw_frames_context_options;
    int hw_frames_context_options_count;
};

struct Encoder;

struct Encoder *encoder_alloc(struct EncoderAllocData setup_data);
void encoder_free(struct Encoder **encoder);