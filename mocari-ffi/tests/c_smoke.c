#include "mocari.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>

#if INTPTR_MAX == INT64_MAX
_Static_assert(sizeof(MocariVertex) == 16, "unexpected MocariVertex layout");
_Static_assert(sizeof(MocariMeshView) == 96, "unexpected MocariMeshView layout");
_Static_assert(sizeof(MocariTextureView) == 24, "unexpected MocariTextureView layout");
#endif
_Static_assert(sizeof(MocariResult) == 4, "MocariResult must be int32_t");

static int fail(const char *operation) {
    fprintf(stderr, "%s failed: %s\n", operation, mocari_last_error_message());
    return 1;
}

static int fail_and_destroy(MocariModelHandle *model, const char *operation) {
    int result = fail(operation);
    mocari_model_destroy(model);
    return result;
}

static int run_model(const char *path) {
    MocariModelHandle *model = mocari_model_create(path);
    if (model == NULL) {
        return fail("mocari_model_create");
    }

    if (mocari_model_set_parameter(model, "ParamAngleX", 12.5f) != MOCARI_OK) {
        return fail_and_destroy(model, "mocari_model_set_parameter");
    }

    float parameter = 0.0f;
    if (mocari_model_get_parameter(model, "ParamAngleX", &parameter) != MOCARI_OK
        || fabsf(parameter - 12.5f) > 0.0001f) {
        return fail_and_destroy(model, "mocari_model_get_parameter");
    }

    if (mocari_model_update(model) != MOCARI_OK) {
        return fail_and_destroy(model, "mocari_model_update");
    }

    size_t mesh_count = 0;
    if (mocari_model_mesh_count(model, &mesh_count) != MOCARI_OK || mesh_count == 0) {
        return fail_and_destroy(model, "mocari_model_mesh_count");
    }

    MocariMeshView mesh;
    if (mocari_model_get_mesh(model, 0, &mesh) != MOCARI_OK
        || mesh.vertices == NULL || mesh.vertex_count == 0
        || mesh.indices == NULL || mesh.index_count == 0) {
        return fail_and_destroy(model, "mocari_model_get_mesh");
    }
    for (size_t i = 0; i < mesh.index_count; ++i) {
        if ((size_t)mesh.indices[i] >= mesh.vertex_count) {
            fprintf(stderr, "mesh index is out of range\n");
            mocari_model_destroy(model);
            return 1;
        }
    }

    size_t texture_count = 0;
    if (mocari_model_texture_count(model, &texture_count) != MOCARI_OK
        || texture_count == 0) {
        return fail_and_destroy(model, "mocari_model_texture_count");
    }

    MocariTextureView texture;
    if (mocari_model_get_texture(model, 0, &texture) != MOCARI_OK
        || texture.rgba == NULL || texture.width == 0 || texture.height == 0
        || texture.byte_count != (size_t)texture.width * texture.height * 4) {
        return fail_and_destroy(model, "mocari_model_get_texture");
    }

    printf("meshes=%zu textures=%zu vertices=%zu indices=%zu\n",
           mesh_count, texture_count, mesh.vertex_count, mesh.index_count);
    mocari_model_destroy(model);
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: c_smoke <model3.json>\n");
        return 2;
    }

    for (int iteration = 0; iteration < 10; ++iteration) {
        int result = run_model(argv[1]);
        if (result != 0) {
            return result;
        }
    }
    return 0;
}
