#define _GNU_SOURCE 1
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <wchar.h>

int fx_printf_formats(char *out, int size)
{
    return snprintf(out, (size_t)size, "%zu|%lld|%p|%s|%c|%x|%o|%a|%.2e|%g", (size_t)42, (long long)-9,
                    (void *)0x1234, "str", 'z', 255, 8, 1.0, 1234.5, 0.0001);
}

int fx_printf_positional(char *out, int size)
{
    return snprintf(out, (size_t)size, "%2$s=%1$d|%3$.*4$f", 7, "seven", 2.5, 3);
}

int fx_stdio_roundtrip(const char *path)
{
    FILE *file = fopen(path, "w+");
    if (file == NULL) return -errno;
    if (fprintf(file, "%d|%s|%.2f|%c\n", 7, "seven", 2.5, 'x') < 0) {
        fclose(file);
        return -100;
    }
    if (fflush(file) != 0) {
        fclose(file);
        return -101;
    }
    rewind(file);
    int number = 0;
    char text[16];
    double real = 0.0;
    char letter = 0;
    int scanned = fscanf(file, "%d|%15[^|]|%lf|%c", &number, text, &real, &letter);
    long position = ftell(file);
    if (fclose(file) != 0) return -102;
    if (scanned != 4) return -103;
    if (number != 7) return -104;
    if (strcmp(text, "seven") != 0) return -105;
    if (real < 2.499 || real > 2.501) return -106;
    if (letter != 'x') return -107;
    if (position != 14) return -108;
    return 1;
}

int fx_stdio_getline(const char *path)
{
    FILE *file = fopen(path, "w");
    if (file == NULL) return -errno;
    fputs("alpha\n", file);
    fputs("beta\n", file);
    fclose(file);
    file = fopen(path, "r");
    if (file == NULL) return -errno;
    char *line = NULL;
    size_t capacity = 0;
    ssize_t first = getline(&line, &capacity, file);
    ssize_t second = getline(&line, &capacity, file);
    ssize_t third = getline(&line, &capacity, file);
    int matches = first == 6 && second == 5 && third == -1 && strcmp(line, "beta\n") == 0;
    free(line);
    fclose(file);
    return matches ? 1 : 0;
}

int fx_stdio_ungetc(const char *path)
{
    FILE *file = fopen(path, "w+");
    if (file == NULL) return -errno;
    fwrite("abc", 1, 3, file);
    rewind(file);
    int first = fgetc(file);
    ungetc(first, file);
    int again = fgetc(file);
    int second = fgetc(file);
    int at_eof = feof(file);
    fclose(file);
    return (first == 'a' && again == 'a' && second == 'b' && !at_eof) ? 1 : 0;
}

int fx_scanf_sscanf(const char *text)
{
    int decimal = 0;
    int detected = 0;
    unsigned int hexadecimal = 0;
    unsigned int octal = 0;
    unsigned int unsign = 0;
    float single = 0.0f;
    double wide = 0.0;
    char word[16];
    char letter = 0;
    int count = sscanf(text, "%d %i %x %o %u %f %lf %9s %c", &decimal, &detected, &hexadecimal, &octal,
                       &unsign, &single, &wide, word, &letter);
    if (count != 9) return -count;
    int mask = 0;
    if (decimal == 42) mask |= 1;
    if (detected == -17) mask |= 2;
    if (hexadecimal == 255) mask |= 4;
    if (octal == 15) mask |= 8;
    if (unsign == 4000000000u) mask |= 16;
    if (single > 2.49f && single < 2.51f) mask |= 32;
    if (wide > 3.24 && wide < 3.26) mask |= 64;
    if (strcmp(word, "word") == 0) mask |= 128;
    if (letter == 'Q') mask |= 256;
    return mask;
}

int fx_scanf_width(void)
{
    int value = 0;
    char text[8];
    int count = sscanf("123 abcdef", "%3d %3s", &value, text);
    return count == 2 && value == 123 && strcmp(text, "abc") == 0 ? 1 : 0;
}

int fx_scanf_count_pointer(void)
{
    int used = -1;
    int count = sscanf("ab12", "ab%n", &used);
    return count == 0 && used == 2 ? 1 : 0;
}

int fx_dirent_count(const char *path)
{
    DIR *directory = opendir(path);
    if (directory == NULL) return -errno;
    int count = 0;
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        count += 1;
    }
    if (closedir(directory) != 0) return -1;
    return count;
}

int fx_stat_size(const char *path)
{
    struct stat status;
    if (stat(path, &status) != 0) return -errno;
    if (!S_ISREG(status.st_mode)) return -100;
    return (int)status.st_size;
}

int fx_mmap_anon(void)
{
    size_t length = 65536;
    unsigned char *region = mmap(NULL, length, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (region == MAP_FAILED) return -errno;
    for (size_t index = 0; index < length; index += 4096) region[index] = (unsigned char)(index / 4096);
    int sum = 0;
    for (size_t index = 0; index < length; index += 4096) sum += region[index];
    if (mprotect(region, length, PROT_READ) != 0) {
        munmap(region, length);
        return -200;
    }
    if (munmap(region, length) != 0) return -201;
    return sum;
}

int fx_mmap_file(const char *path)
{
    int descriptor = open(path, O_RDONLY);
    if (descriptor < 0) return -errno;
    char *region = mmap(NULL, 5, PROT_READ, MAP_PRIVATE, descriptor, 0);
    if (region == MAP_FAILED) {
        close(descriptor);
        return -errno;
    }
    int matches = memcmp(region, "hello", 5) == 0;
    munmap(region, 5);
    close(descriptor);
    return matches ? 1 : 0;
}

int fx_maps_contains(const char *needle)
{
    int descriptor = open("/proc/self/maps", O_RDONLY);
    if (descriptor < 0) return -errno;
    char buffer[65536];
    ssize_t length = read(descriptor, buffer, sizeof(buffer) - 1);
    close(descriptor);
    if (length <= 0) return -100;
    buffer[length] = 0;
    int count = 0;
    char *cursor = buffer;
    while ((cursor = strstr(cursor, needle)) != NULL) {
        count += 1;
        cursor += strlen(needle);
    }
    return count;
}

int fx_swprintf_wide(unsigned int *out, int capacity)
{
    if (capacity < 8) return -1;
    wchar_t buffer[16];
    int written = swprintf(buffer, 16, L"%d-%ls", 5, L"wide");
    if (written != 6) return -written;
    for (int index = 0; index <= written; index += 1) out[index] = (unsigned int)buffer[index];
    return written;
}

int fx_tmpfile_roundtrip(void)
{
    FILE *file = tmpfile();
    if (file == NULL) return -errno;
    if (fwrite("tmp", 1, 3, file) != 3) {
        fclose(file);
        return -100;
    }
    fseek(file, 0, SEEK_SET);
    char buffer[4] = {0};
    size_t got = fread(buffer, 1, 3, file);
    fclose(file);
    return got == 3 && memcmp(buffer, "tmp", 3) == 0 ? 1 : 0;
}
