extern void __fx_record(int value);

__asm__(
    ".section .text.fx_pool,\"ax\"\n"
    ".global fx_pool_words\n"
    ".balign 4\n"
    "fx_pool_words:\n"
    ".word 0xD53BD040\n"
    ".word 0xF800865E\n"
    ".word 0xF85F8E5E\n"
    ".text\n");

extern unsigned int fx_pool_words[];

__attribute__((constructor(101))) static void child_first_constructor(void)
{
    __fx_record(1);
}

__attribute__((constructor(102))) static void child_second_constructor(void)
{
    __fx_record(2);
}

__attribute__((destructor(101))) static void child_first_destructor(void)
{
    __fx_record(-1);
}

__attribute__((destructor(102))) static void child_second_destructor(void)
{
    __fx_record(-2);
}

unsigned int fx_pool_word(int index)
{
    return fx_pool_words[index];
}

void fx_write_x18(void)
{
    __asm__ volatile(".inst 0xAA0003F2");
}
