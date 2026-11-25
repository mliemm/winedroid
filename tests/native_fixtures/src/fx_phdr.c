// cite: bionic link.h#dl_phdr_info, ElfW
// cite: bionic elf.h#PT_LOAD, PT_GNU_EH_FRAME

#include <elf.h>
#include <link.h>

struct fx_phdr_entry {
    unsigned long addr;
    const char *name;
    unsigned long phnum;
    unsigned long first_load_vaddr;
    unsigned long first_load_memsz;
    unsigned long gnu_eh_frame_vaddr;
    unsigned long long adds;
    unsigned long long subs;
    unsigned long tls_modid;
    void *tls_data;
};

struct fx_phdr_state {
    struct fx_phdr_entry *entries;
    int capacity;
    int count;
    int stop_after;
};

static __thread int fx_phdr_tls = 7;

static int record_callback(struct dl_phdr_info *info, size_t size, void *data)
{
    struct fx_phdr_state *state = (struct fx_phdr_state *)data;
    if (state->count >= state->capacity) {
        return 0;
    }
    struct fx_phdr_entry *entry = &state->entries[state->count];
    entry->addr = (unsigned long)info->dlpi_addr;
    entry->name = info->dlpi_name;
    entry->phnum = (unsigned long)info->dlpi_phnum;
    entry->first_load_vaddr = 0;
    entry->first_load_memsz = 0;
    entry->gnu_eh_frame_vaddr = 0;
    entry->adds = (unsigned long long)info->dlpi_adds;
    entry->subs = (unsigned long long)info->dlpi_subs;
    entry->tls_modid = (unsigned long)info->dlpi_tls_modid;
    entry->tls_data = info->dlpi_tls_data;
    const ElfW(Phdr) *phdr = info->dlpi_phdr;
    if (phdr != 0) {
        int found_load = 0;
        for (unsigned long index = 0; index < entry->phnum; ++index) {
            if (phdr[index].p_type == PT_LOAD && found_load == 0) {
                entry->first_load_vaddr = (unsigned long)phdr[index].p_vaddr;
                entry->first_load_memsz = (unsigned long)phdr[index].p_memsz;
                found_load = 1;
            }
            if (phdr[index].p_type == PT_GNU_EH_FRAME) {
                entry->gnu_eh_frame_vaddr = (unsigned long)phdr[index].p_vaddr;
            }
        }
    }
    state->count += 1;
    if (state->stop_after != 0 && state->count == state->stop_after) {
        return 7;
    }
    return 0;
}

int fx_phdr_record(struct fx_phdr_entry *entries, int capacity)
{
    struct fx_phdr_state state;
    state.entries = entries;
    state.capacity = capacity;
    state.count = 0;
    state.stop_after = capacity == 2 ? 2 : 0;
    int result = dl_iterate_phdr(record_callback, &state);
    if (result != 0) {
        return result;
    }
    return state.count;
}

int fx_phdr_touch_tls(void)
{
    fx_phdr_tls += 1;
    return fx_phdr_tls;
}

int fx_phdr_tls_value(void)
{
    return fx_phdr_tls;
}
