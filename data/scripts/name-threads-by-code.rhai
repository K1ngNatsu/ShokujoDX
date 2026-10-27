//! Name Threads by Code
//! Puts the thread code before the name of each thread that has a code,
//! for example "1747 Red", so the operator finds the spool quickly.

let changed = 0;
for t in threads() {
    if t.code != "" && !t.name.starts_with(t.code) {
        set_thread_name(t.id, `${t.code} ${t.name}`);
        changed += 1;
    }
}
print(`${changed} threads renamed.`);
