#ifndef MOCARI_H
#define MOCARI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MocariModelHandle MocariModelHandle;

typedef int32_t MocariResult;

#define MOCARI_OK ((MocariResult)0)
#define MOCARI_NULL_ARGUMENT ((MocariResult)1)
#define MOCARI_INVALID_UTF8 ((MocariResult)2)
#define MOCARI_INVALID_HANDLE ((MocariResult)3)
#define MOCARI_NOT_FOUND ((MocariResult)4)
#define MOCARI_INVALID_OUTPUT ((MocariResult)5)
#define MOCARI_RUNTIME_ERROR ((MocariResult)6)

typedef struct MocariVertex {
    float position[2];
    float uv[2];
} MocariVertex;

typedef struct MocariMeshView {
    int32_t texture_index;
    uint8_t drawable_flags;
    uint8_t is_inverted_mask;
    uint8_t reserved[2];
    float opacity;
    float draw_order;
    int32_t render_order;
    float multiply_color[3];
    float screen_color[3];
    const MocariVertex *vertices;
    size_t vertex_count;
    const uint16_t *indices;
    size_t index_count;
    const int32_t *masks;
    size_t mask_count;
} MocariMeshView;

typedef struct MocariTextureView {
    uint32_t width;
    uint32_t height;
    const uint8_t *rgba;
    size_t byte_count;
} MocariTextureView;

/*
 * MocariModelHandle and all arrays referenced by MocariMeshView are owned by
 * Rust. Callers must not free them. Mesh pointers remain valid only until the
 * next mocari_model_update() or mocari_model_destroy() for that handle.
 * Texture pixel pointers are also owned by Rust and remain valid until
 * mocari_model_destroy() for that handle.
 *
 * Each non-null handle returned by mocari_model_create() must be passed to
 * mocari_model_destroy() exactly once. After destroy, the handle and every
 * pointer obtained from it are invalid. Calling a model function with a
 * destroyed handle or destroying the same non-null handle twice is undefined
 * behavior. Set the caller's handle to NULL immediately after destroy.
 *
 * A handle is not thread-safe. The caller must serialize all operations on a
 * handle and must not use mesh pointers concurrently with update or destroy.
 * mocari_last_error_message() is thread-local and remains valid until the next
 * Mocari FFI call on the same thread.
 */

const char *mocari_last_error_message(void);
MocariModelHandle *mocari_model_create(const char *path);
void mocari_model_destroy(MocariModelHandle *handle);
MocariResult mocari_model_set_parameter(MocariModelHandle *handle, const char *id, float value);
MocariResult mocari_model_get_parameter(const MocariModelHandle *handle, const char *id, float *value);
MocariResult mocari_model_update(MocariModelHandle *handle);
MocariResult mocari_model_mesh_count(const MocariModelHandle *handle, size_t *count);
MocariResult mocari_model_get_mesh(const MocariModelHandle *handle, size_t index, MocariMeshView *output);
MocariResult mocari_model_texture_count(const MocariModelHandle *handle, size_t *count);
MocariResult mocari_model_get_texture(const MocariModelHandle *handle, size_t index, MocariTextureView *output);

#ifdef __cplusplus
}
#endif

#endif
