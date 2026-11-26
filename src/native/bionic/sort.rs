use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::export::in_libc;

struct Sorter {
    comparator: usize,
    size: usize,
    failure: Option<String>,
}

impl Sorter {
    fn compare(&mut self, left: usize, right: usize) -> i32 {
        if self.failure.is_some() {
            return 0;
        }
        match CallBuilder::new(self.comparator)
            .ptr(left)
            .ptr(right)
            .invoke()
        {
            Ok(result) => result.i32(),
            Err(error) => {
                self.failure = Some(error.to_string());
                0
            }
        }
    }

    fn swap(&self, left: usize, right: usize, length: usize) {
        if length > 0 {
            for offset in 0..length {
                unsafe { std::ptr::swap((left + offset) as *mut u8, (right + offset) as *mut u8) };
            }
        }
    }

    fn median(&mut self, a: usize, b: usize, c: usize) -> usize {
        // cite: bionic qsort.c#med3
        if self.compare(a, b) < 0 {
            if self.compare(b, c) < 0 {
                b
            } else if self.compare(a, c) < 0 {
                c
            } else {
                a
            }
        } else if self.compare(b, c) > 0 {
            b
        } else if self.compare(a, c) < 0 {
            a
        } else {
            c
        }
    }

    fn insertion(&mut self, base: usize, count: usize) {
        let size = self.size;
        let mut middle = base + size;
        while middle < base + count * size {
            let mut low = middle;
            while low > base && self.compare(low - size, low) > 0 {
                self.swap(low, low - size, size);
                low -= size;
            }
            middle += size;
        }
    }

    fn sort(&mut self, mut base: usize, mut count: usize) {
        // cite: bionic qsort.c#qsort
        let size = self.size;
        loop {
            if self.failure.is_some() {
                return;
            }
            if count < 7 {
                self.insertion(base, count);
                return;
            }
            let mut middle = base + (count / 2) * size;
            if count > 7 {
                let mut low = base;
                let mut high = base + (count - 1) * size;
                if count > 40 {
                    let step = (count / 8) * size;
                    low = self.median(low, low + step, low + 2 * step);
                    middle = self.median(middle - step, middle, middle + step);
                    high = self.median(high - 2 * step, high - step, high);
                }
                middle = self.median(low, middle, high);
            }
            self.swap(base, middle, size);
            let mut pa = base + size;
            let mut pb = pa;
            let mut pc = base + (count - 1) * size;
            let mut pd = pc;
            let mut swapped = false;
            loop {
                while pb <= pc {
                    let result = self.compare(pb, base);
                    if result > 0 {
                        break;
                    }
                    if result == 0 {
                        swapped = true;
                        self.swap(pa, pb, size);
                        pa += size;
                    }
                    pb += size;
                }
                while pb <= pc {
                    let result = self.compare(pc, base);
                    if result < 0 {
                        break;
                    }
                    if result == 0 {
                        swapped = true;
                        self.swap(pc, pd, size);
                        pd -= size;
                    }
                    pc -= size;
                }
                if pb > pc {
                    break;
                }
                self.swap(pb, pc, size);
                swapped = true;
                pb += size;
                pc -= size;
            }
            if !swapped {
                self.insertion(base, count);
                return;
            }
            let end = base + count * size;
            let mut span = (pa - base).min(pb - pa);
            self.swap(base, pb - span, span);
            span = (pd - pc).min(end - pd - size);
            self.swap(pb, end - span, span);
            let left = pb - pa;
            let right = pd - pc;
            if left <= right {
                if left > size {
                    self.sort(base, left / size);
                }
                if right > size {
                    base = end - right;
                    count = right / size;
                    continue;
                }
            } else {
                if right > size {
                    self.sort(end - right, right / size);
                }
                if left > size {
                    count = left / size;
                    continue;
                }
            }
            return;
        }
    }
}

fn qsort(context: &mut CallContext) {
    let (base, count, size, comparator) = (
        context.arg_ptr(0),
        context.arg_u64(1) as usize,
        context.arg_u64(2) as usize,
        context.arg_ptr(3),
    );
    if count == 0 || size == 0 {
        return;
    }
    let mut sorter = Sorter {
        comparator,
        size,
        failure: None,
    };
    sorter.sort(base, count);
    if let Some(message) = sorter.failure {
        context.fail(message);
    }
}

fn bsearch(context: &mut CallContext) {
    // cite: bionic bsearch.c#bsearch
    let (key, mut base, count, size, comparator) = (
        context.arg_ptr(0),
        context.arg_ptr(1),
        context.arg_u64(2) as usize,
        context.arg_u64(3) as usize,
        context.arg_ptr(4),
    );
    let mut limit = count;
    while limit != 0 {
        let probe = base + (limit >> 1) * size;
        match CallBuilder::new(comparator).ptr(key).ptr(probe).invoke() {
            Ok(result) => {
                let order = result.i32();
                if order == 0 {
                    context.ret_ptr(probe);
                    return;
                }
                if order > 0 {
                    base = probe + size;
                    limit -= 1;
                }
            }
            Err(error) => return context.fail(error.to_string()),
        }
        limit >>= 1;
    }
    context.ret_ptr(0);
}

pub fn register() {
    in_libc("qsort", qsort);
    in_libc("bsearch", bsearch);
}
