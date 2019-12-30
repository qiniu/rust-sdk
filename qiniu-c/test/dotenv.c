#include <stdio.h>
#include <memory.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

/* strtok_r() won't remove the whole ${ part, only the $ */
#define remove_bracket(name) name + 1

#define remove_space(value) value + 1


static char *concat(char *buffer, char *string)
{
    if (!buffer) {
        return strdup(string);
    }
    if (string) {
        size_t length = strlen(buffer) + strlen(string) + 1;
        char *new = realloc(buffer, length);

        return strcat(new, string);
    }

    return buffer;
}

static bool is_nested(char *value)
{
    return strstr(value, "${") && strstr(value, "}");
}

/**
 * @example With TEST_DIR=${BASE_DIR}/.test the first strtok_r call will return
 * BASE_DIR}/.test instead of NULL, or an empty string
 */
static char *prepare_value(char *value)
{
    char *new = malloc(strlen(value) + 2);
    sprintf(new, " %s", value);

    return new;
}

static char *parse_value(char *value)
{
    value = prepare_value(value);

    char *search = value, *parsed = NULL, *tok_ptr;
    char *name;

    if (value && is_nested(value)) {
        while (1) {
#if defined(_WIN32) || defined(WIN32)
            parsed = concat(parsed, strtok_s(search, "${", &tok_ptr));
            name = strtok_s(NULL, "}", &tok_ptr);
#else
            parsed = concat(parsed, strtok_r(search, "${", &tok_ptr));
            name = strtok_r(NULL, "}", &tok_ptr);
#endif

            if (!name) {
                break;
            }
            parsed = concat(parsed, getenv(remove_bracket(name)));
            search = NULL;
        }
        free(value);

        return parsed;
    }
    return value;
}

static bool is_commented(char *line)
{
    if ('#' == line[0]) {
        return true;
    }

    int i = 0;
    while (' ' == line[i]) {
        if ('#' == line[++i]) {
            return true;
        }
    }

    return false;
}

static void set_variable(char *name, char *original, bool overwrite)
{
    char *parsed;

    if (original) {
        parsed = parse_value(original);
#if defined(_WIN32) || defined(WIN32)
        if (overwrite != 0 || getenv(name) == NULL) {
            _putenv_s(name, remove_space(parsed), overwrite);
        }
#else
        setenv(name, remove_space(parsed), overwrite);
#endif

        free(parsed);
    }
}

static void parse(FILE *file, bool overwrite)
{
    const int BUF_SIZE = 4096;
    char *name, *original, *line = (char *) malloc(BUF_SIZE), *tok_ptr = NULL;
    memset(line, 0, BUF_SIZE);

    while (fgets(line, BUF_SIZE, file) != NULL) {
        if (!is_commented(line)) {
#if defined(_WIN32) || defined(WIN32)
            name = strtok_s(line, "=", &tok_ptr);
            original = strtok_s(NULL, "\n", &tok_ptr);
#else
            name = strtok_r(line, "=", &tok_ptr);
            original = strtok_r(NULL, "\n", &tok_ptr);
#endif

            set_variable(name, original, overwrite);
        }
    }
    free(line);
}

static FILE *open_default(char *base_path)
{
    char *path = (char *) malloc(strlen(base_path) + strlen(".env") + 1);
    sprintf(path, "%s/.env", base_path);

    FILE *file = fopen(path, "rb");
    free((void *) path);
    return file;
}

int env_load(char *path, bool overwrite)
{
    FILE *file = open_default(path);

    if (!file) {
        file = fopen(path, "rb");

        if (!file) {
            return -1;
        }
    }
    parse(file, overwrite);
    fclose(file);

    return 0;
}
