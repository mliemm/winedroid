int fx_relro_plain = 5;

int fx_relro_target(void)
{
    return 42;
}

int (*const fx_relro_table[4096])(void) = {[0 ... 4095] = fx_relro_target};

int fx_relro_call(int index)
{
    return fx_relro_table[index]();
}
