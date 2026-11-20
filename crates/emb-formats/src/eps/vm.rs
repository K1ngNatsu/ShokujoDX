//! The machine of the PostScript reader (ADR 0138): the objects, the
//! memory, the three stacks and the run loop. The rules are those of the
//! PostScript language reference (third edition): an executable array that
//! the machine finds in a procedure goes on the operand stack; an
//! executable name runs its value.
//!
//! All composite objects (strings, arrays, dictionaries) are in tables of
//! the machine, and an object has the index of its table entry. So the
//! objects are small values, a dictionary can contain itself, and all the
//! memory goes when the reader stops. The machine has limits for the
//! steps, the memory, the stacks and the errors, so a bad file stops.

use std::collections::HashMap;
use std::rc::Rc;

use super::graphics::{Drawing, GState};

/// The limits of one run: the steps and the memory (bytes of strings,
/// arrays and dictionaries).
#[derive(Debug, Clone, Copy)]
pub(super) struct Limits {
    pub steps: u64,
    pub memory: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            steps: 40_000_000,
            memory: 256 << 20,
        }
    }
}

/// The most errors outside `stopped` before the run stops.
pub(super) const MAX_ERRORS: usize = 2_000;
const MAX_OPERANDS: usize = 100_000;
const MAX_FRAMES: usize = 20_000;
pub(super) const MAX_DICT_STACK: usize = 1_000;
/// The most procedures inside procedures in the text of a file.
pub(super) const MAX_PROC_DEPTH: usize = 2_000;

/// A part of a string or an array: the table entry, the start and the length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Slice {
    pub id: u32,
    pub start: u32,
    pub len: u32,
}

impl Slice {
    pub(super) fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }
}

/// A PostScript object. The `bool` of a name, a string, an array or a file
/// is its executable attribute.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Obj {
    Null,
    Bool(bool),
    Int(i32),
    Real(f64),
    Name(u32, bool),
    Str(Slice, bool),
    Arr(Slice, bool),
    Dict(u32),
    Op(u16),
    Mark,
    File(u32, bool),
    Save(u32),
    Font(u32),
    GState(u32),
}

/// A key of a dictionary. A string key is a name; an integer real is an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Key {
    Name(u32),
    Int(i64),
    Real(u64),
    Bool(bool),
    Other(u8, u32, u32, u32),
}

/// A dictionary: the entries in the order that they were made (so `forall`
/// gives the same order each time) and an index.
#[derive(Debug, Default)]
pub(super) struct DictData {
    pub index: HashMap<Key, usize>,
    pub entries: Vec<(Obj, Obj)>,
    pub cap: usize,
}

/// A part of a run that is not finished.
#[derive(Debug, Clone)]
pub(super) enum Frame {
    /// The text of a file or of an executable string.
    Tokens {
        src: u32,
        pos: usize,
    },
    Proc {
        arr: Slice,
        pos: u32,
    },
    Loop {
        proc: Obj,
    },
    Repeat {
        proc: Obj,
        left: i64,
    },
    ForInt {
        proc: Obj,
        cur: i64,
        inc: i64,
        lim: i64,
    },
    ForReal {
        proc: Obj,
        cur: f64,
        inc: f64,
        lim: f64,
    },
    ForallArr {
        proc: Obj,
        arr: Slice,
        i: u32,
    },
    ForallStr {
        proc: Obj,
        s: Slice,
        i: u32,
    },
    ForallDict {
        proc: Obj,
        dict: u32,
        i: usize,
    },
    Stopped,
    /// What to do when the procedure above it is done.
    Then(Cont),
}

/// The work after a procedure that an operator runs.
#[derive(Debug, Clone)]
pub(super) enum Cont {
    /// Restore the graphics state (after the paint procedure of a form).
    GRestore,
    /// Take the values of a tint transform or of an indexed color
    /// procedure: `n` values in the space `alt`.
    Tint { n: usize, alt: super::graphics::Base },
    /// Read the data of a picture from procedures until `left` bytes came.
    Picture {
        procs: Vec<Obj>,
        next: usize,
        left: i64,
        first: bool,
    },
}

/// A PostScript error.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PsError {
    StackUnderflow,
    StackOverflow,
    TypeCheck,
    RangeCheck,
    Undefined(u32),
    UndefinedResult,
    UndefinedResource,
    InvalidExit,
    InvalidAccess,
    LimitCheck,
    UnmatchedMark,
    IoError,
    /// Not in this reader.
    Unsupported,
    // These stop the run.
    VmError,
    TooManySteps,
    TooManyErrors,
    /// The program painted `graphics::MAX_SHAPES` paths.
    TooManyShapes,
    /// The end of the run (`quit`, or the end of the first page).
    Quit,
}

impl PsError {
    pub(super) fn stops_the_run(&self) -> bool {
        matches!(
            self,
            PsError::VmError | PsError::TooManySteps | PsError::TooManyErrors | PsError::TooManyShapes | PsError::Quit
        )
    }

    /// The name of the error in PostScript.
    pub(super) fn name(&self) -> &'static str {
        match self {
            PsError::StackUnderflow => "stackunderflow",
            PsError::StackOverflow => "stackoverflow",
            PsError::TypeCheck => "typecheck",
            PsError::RangeCheck => "rangecheck",
            PsError::Undefined(_) => "undefined",
            PsError::UndefinedResult => "undefinedresult",
            PsError::UndefinedResource => "undefinedresource",
            PsError::InvalidExit => "invalidexit",
            PsError::InvalidAccess => "invalidaccess",
            PsError::LimitCheck => "limitcheck",
            PsError::UnmatchedMark => "unmatchedmark",
            PsError::IoError => "ioerror",
            PsError::Unsupported => "unsupported",
            PsError::VmError => "VMerror",
            PsError::TooManySteps => "timeout",
            PsError::TooManyErrors => "toomanyerrors",
            PsError::TooManyShapes => "toomanyshapes",
            PsError::Quit => "quit",
        }
    }
}

pub(super) type R<T = ()> = Result<T, PsError>;
pub(super) type OpFn = fn(&mut Vm) -> R;

/// A file object: the main file (0), the standard output (1) or a filter.
/// A filter decodes its source when it is read first (as in PostScript:
/// `currentfile /ASCII85Decode filter` in a dictionary reads nothing until
/// the operator that uses the dictionary reads the data after it).
#[derive(Debug, Default)]
pub(super) struct FileData {
    pub data: Vec<u8>,
    pub pos: usize,
    pub pending: Option<Pending>,
}

/// A filter that has not read its source yet.
#[derive(Debug, Clone)]
pub(super) struct Pending {
    pub kind: String,
    pub source: Obj,
    /// The count and the end string of `SubFileDecode`.
    pub count: usize,
    pub eod: Vec<u8>,
}

/// One entry of the graphics state stack: `true` if `save` made it.
#[derive(Debug, Clone)]
pub(super) struct Pushed {
    pub g: GState,
    pub by_save: bool,
}

/// The PostScript machine.
pub(super) struct Vm {
    pub ops: Vec<Obj>,
    pub exec: Vec<Frame>,
    pub dstack: Vec<u32>,
    names: Vec<Box<[u8]>>,
    name_ids: HashMap<Box<[u8]>, u32>,
    pub strings: Vec<Vec<u8>>,
    pub arrays: Vec<Vec<Obj>>,
    pub dicts: Vec<DictData>,
    pub sources: Vec<Rc<[u8]>>,
    pub files: Vec<FileData>,
    pub op_fns: Vec<OpFn>,
    pub op_names: Vec<u32>,
    pub systemdict: u32,
    pub userdict: u32,
    pub globaldict: u32,
    pub fontdir: u32,
    pub resources: u32,
    pub dollar_error: u32,
    pub g: GState,
    pub gstack: Vec<Pushed>,
    /// For each `save`: the length of the graphics state stack.
    pub saves: Vec<usize>,
    pub gstates: Vec<GState>,
    pub out: Drawing,
    pub steps: u64,
    memory: usize,
    limits: Limits,
    pub errors: usize,
    pub first_error: Option<String>,
    /// The operators of the AI script: an unknown name clears the operand
    /// stack (each operator of the script takes all the operands before it).
    pub ai_script: bool,
    pub ai: super::ai::AiState,
    pub seed: u32,
}

impl Vm {
    /// A machine that runs `program`. `ai_script`: with the operators of
    /// the AI script on top of the dictionary stack.
    pub(super) fn new(program: Vec<u8>, ai_script: bool, limits: Limits) -> Vm {
        let mut vm = Vm {
            ops: Vec::new(),
            exec: Vec::new(),
            dstack: Vec::new(),
            names: Vec::new(),
            name_ids: HashMap::new(),
            strings: Vec::new(),
            arrays: Vec::new(),
            dicts: Vec::new(),
            sources: vec![Rc::from(program)],
            files: vec![FileData::default(), FileData::default()],
            op_fns: Vec::new(),
            op_names: Vec::new(),
            systemdict: 0,
            userdict: 0,
            globaldict: 0,
            fontdir: 0,
            resources: 0,
            dollar_error: 0,
            g: GState::default(),
            gstack: Vec::new(),
            saves: Vec::new(),
            gstates: Vec::new(),
            out: Drawing::default(),
            steps: 0,
            memory: 0,
            limits,
            errors: 0,
            first_error: None,
            ai_script,
            ai: Default::default(),
            seed: 1,
        };
        vm.setup();
        vm.exec.push(Frame::Tokens { src: 0, pos: 0 });
        vm
    }

    fn setup(&mut self) {
        let system = self.new_dict_raw(512);
        let global = self.new_dict_raw(64);
        let user = self.new_dict_raw(256);
        self.systemdict = system;
        self.globaldict = global;
        self.userdict = user;
        self.dstack = vec![system, global, user];
        for table in [super::ops::CORE, super::graphics::GRAPHICS] {
            for &(name, f) in table {
                let id = self.op_fns.len() as u16;
                self.op_fns.push(f);
                let n = self.intern(name.as_bytes());
                self.op_names.push(n);
                self.put_raw(system, Key::Name(n), Obj::Name(n, false), Obj::Op(id));
            }
        }
        let consts = [
            ("true", Obj::Bool(true)),
            ("false", Obj::Bool(false)),
            ("null", Obj::Null),
        ];
        for (name, v) in consts {
            let n = self.intern(name.as_bytes());
            self.put_raw(system, Key::Name(n), Obj::Name(n, false), v);
        }
        self.fontdir = self.new_dict_raw(64);
        self.resources = self.new_dict_raw(16);
        let error_dict = self.new_dict_raw(32);
        self.dollar_error = self.new_dict_raw(16);
        let statusdict = self.new_dict_raw(32);
        for (name, d) in [
            ("systemdict", system),
            ("userdict", user),
            ("globaldict", global),
            ("shareddict", global),
            ("FontDirectory", self.fontdir),
            ("GlobalFontDirectory", self.fontdir),
            ("SharedFontDirectory", self.fontdir),
            ("errordict", error_dict),
            ("$error", self.dollar_error),
            ("statusdict", statusdict),
        ] {
            let n = self.intern(name.as_bytes());
            self.put_raw(system, Key::Name(n), Obj::Name(n, false), Obj::Dict(d));
        }
        let newerror = self.intern(b"newerror");
        self.put_raw(
            self.dollar_error,
            Key::Name(newerror),
            Obj::Name(newerror, false),
            Obj::Bool(false),
        );
        // The encodings: 256 names each. The reader does not draw text, so
        // the names are all `.notdef`.
        let notdef = self.intern(b".notdef");
        for name in ["StandardEncoding", "ISOLatin1Encoding"] {
            let id = self.arrays.len() as u32;
            self.arrays.push(vec![Obj::Name(notdef, false); 256]);
            let n = self.intern(name.as_bytes());
            let a = Obj::Arr(Slice { id, start: 0, len: 256 }, false);
            self.put_raw(system, Key::Name(n), Obj::Name(n, false), a);
        }
        if self.ai_script {
            let ai = self.new_dict_raw(128);
            for &(name, f) in super::ai::AI {
                let id = self.op_fns.len() as u16;
                self.op_fns.push(f);
                let n = self.intern(name.as_bytes());
                self.op_names.push(n);
                self.put_raw(ai, Key::Name(n), Obj::Name(n, false), Obj::Op(id));
            }
            self.dstack.push(ai);
        }
    }

    // ----- Memory -----

    pub(super) fn charge(&mut self, bytes: usize) -> R {
        self.memory = self.memory.saturating_add(bytes);
        if self.memory > self.limits.memory {
            return Err(PsError::VmError);
        }
        Ok(())
    }

    pub(super) fn intern(&mut self, s: &[u8]) -> u32 {
        if let Some(&id) = self.name_ids.get(s) {
            return id;
        }
        let id = self.names.len() as u32;
        self.names.push(s.into());
        self.name_ids.insert(s.into(), id);
        id
    }

    pub(super) fn name_bytes(&self, id: u32) -> &[u8] {
        &self.names[id as usize]
    }

    pub(super) fn name_text(&self, id: u32) -> String {
        String::from_utf8_lossy(self.name_bytes(id)).into_owned()
    }

    pub(super) fn new_string(&mut self, bytes: Vec<u8>) -> R<Slice> {
        self.charge(bytes.len() + 16)?;
        let id = self.strings.len() as u32;
        let len = u32::try_from(bytes.len()).map_err(|_| PsError::LimitCheck)?;
        self.strings.push(bytes);
        Ok(Slice { id, start: 0, len })
    }

    pub(super) fn new_array(&mut self, items: Vec<Obj>) -> R<Slice> {
        self.charge(items.len() * 24 + 16)?;
        let id = self.arrays.len() as u32;
        let len = u32::try_from(items.len()).map_err(|_| PsError::LimitCheck)?;
        self.arrays.push(items);
        Ok(Slice { id, start: 0, len })
    }

    fn new_dict_raw(&mut self, cap: usize) -> u32 {
        let id = self.dicts.len() as u32;
        self.dicts.push(DictData {
            cap,
            ..Default::default()
        });
        id
    }

    pub(super) fn new_dict(&mut self, cap: usize) -> R<u32> {
        self.charge(64)?;
        Ok(self.new_dict_raw(cap.min(1 << 16)))
    }

    pub(super) fn sbytes(&self, s: Slice) -> &[u8] {
        &self.strings[s.id as usize][s.range()]
    }

    pub(super) fn sbytes_mut(&mut self, s: Slice) -> &mut [u8] {
        &mut self.strings[s.id as usize][s.range()]
    }

    pub(super) fn arr(&self, a: Slice) -> &[Obj] {
        &self.arrays[a.id as usize][a.range()]
    }

    pub(super) fn arr_mut(&mut self, a: Slice) -> &mut [Obj] {
        &mut self.arrays[a.id as usize][a.range()]
    }

    // ----- Dictionaries -----

    /// The key of an object (a string is a name).
    pub(super) fn key_of(&mut self, o: Obj) -> R<Key> {
        Ok(match o {
            Obj::Name(n, _) => Key::Name(n),
            Obj::Str(s, _) => {
                let b = self.sbytes(s).to_vec();
                Key::Name(self.intern(&b))
            }
            Obj::Int(i) => Key::Int(i64::from(i)),
            Obj::Real(r) if r.fract() == 0.0 && r.abs() < 9e15 => Key::Int(r as i64),
            Obj::Real(r) => Key::Real(r.to_bits()),
            Obj::Bool(b) => Key::Bool(b),
            Obj::Null => return Err(PsError::TypeCheck),
            Obj::Arr(s, _) => Key::Other(1, s.id, s.start, s.len),
            Obj::Dict(d) => Key::Other(2, d, 0, 0),
            Obj::Op(o) => Key::Other(3, u32::from(o), 0, 0),
            Obj::Mark => Key::Other(4, 0, 0, 0),
            Obj::File(f, _) => Key::Other(5, f, 0, 0),
            Obj::Save(s) => Key::Other(6, s, 0, 0),
            Obj::Font(f) => Key::Other(7, f, 0, 0),
            Obj::GState(g) => Key::Other(8, g, 0, 0),
        })
    }

    fn put_raw(&mut self, d: u32, k: Key, ko: Obj, v: Obj) {
        let dict = &mut self.dicts[d as usize];
        match dict.index.get(&k) {
            Some(&i) => dict.entries[i].1 = v,
            None => {
                dict.index.insert(k, dict.entries.len());
                dict.entries.push((ko, v));
            }
        }
    }

    pub(super) fn dict_put(&mut self, d: u32, key: Obj, v: Obj) -> R {
        let k = self.key_of(key)?;
        if !self.dicts[d as usize].index.contains_key(&k) {
            self.charge(64)?;
        }
        // A string key is kept as a name.
        let ko = match k {
            Key::Name(n) => Obj::Name(n, false),
            _ => key,
        };
        self.put_raw(d, k, ko, v);
        Ok(())
    }

    pub(super) fn dict_get(&mut self, d: u32, key: Obj) -> R<Option<Obj>> {
        let k = self.key_of(key)?;
        Ok(self.dict_get_key(d, k))
    }

    pub(super) fn dict_get_key(&self, d: u32, k: Key) -> Option<Obj> {
        let dict = &self.dicts[d as usize];
        dict.index.get(&k).map(|&i| dict.entries[i].1)
    }

    /// The value of the name `name` in dictionary `d`.
    pub(super) fn dict_get_name(&mut self, d: u32, name: &str) -> Option<Obj> {
        let n = self.intern(name.as_bytes());
        self.dict_get_key(d, Key::Name(n))
    }

    pub(super) fn dict_undef(&mut self, d: u32, key: Obj) -> R {
        let k = self.key_of(key)?;
        let dict = &mut self.dicts[d as usize];
        if let Some(i) = dict.index.remove(&k) {
            dict.entries.remove(i);
            for v in dict.index.values_mut() {
                if *v > i {
                    *v -= 1;
                }
            }
        }
        Ok(())
    }

    /// The value of a name in the dictionary stack, from the top.
    pub(super) fn lookup(&self, name: u32) -> Option<Obj> {
        let k = Key::Name(name);
        self.dstack.iter().rev().find_map(|&d| self.dict_get_key(d, k))
    }

    /// The dictionary of the stack that has `key`, from the top.
    pub(super) fn where_key(&mut self, key: Obj) -> R<Option<u32>> {
        let k = self.key_of(key)?;
        Ok(self
            .dstack
            .iter()
            .rev()
            .copied()
            .find(|&d| self.dicts[d as usize].index.contains_key(&k)))
    }

    pub(super) fn current_dict(&self) -> u32 {
        *self.dstack.last().unwrap_or(&self.userdict)
    }

    /// A literal name.
    pub(super) fn name_obj(&mut self, s: &str) -> Obj {
        Obj::Name(self.intern(s.as_bytes()), false)
    }

    // ----- The operand stack -----

    pub(super) fn push(&mut self, o: Obj) -> R {
        if self.ops.len() >= MAX_OPERANDS {
            return Err(PsError::StackOverflow);
        }
        self.ops.push(o);
        Ok(())
    }

    pub(super) fn pop(&mut self) -> R<Obj> {
        self.ops.pop().ok_or(PsError::StackUnderflow)
    }

    /// The object `i` places under the top (0 is the top), not removed.
    pub(super) fn top(&self, i: usize) -> R<Obj> {
        let n = self.ops.len();
        if i >= n {
            return Err(PsError::StackUnderflow);
        }
        Ok(self.ops[n - 1 - i])
    }

    pub(super) fn need(&self, n: usize) -> R {
        if self.ops.len() < n {
            return Err(PsError::StackUnderflow);
        }
        Ok(())
    }

    pub(super) fn drop_n(&mut self, n: usize) {
        let len = self.ops.len();
        self.ops.truncate(len.saturating_sub(n));
    }

    pub(super) fn push_int(&mut self, v: i64) -> R {
        match i32::try_from(v) {
            Ok(i) => self.push(Obj::Int(i)),
            Err(_) => self.push(Obj::Real(v as f64)),
        }
    }

    pub(super) fn push_real(&mut self, v: f64) -> R {
        if !v.is_finite() {
            return Err(PsError::UndefinedResult);
        }
        self.push(Obj::Real(v))
    }

    pub(super) fn push_bool(&mut self, v: bool) -> R {
        self.push(Obj::Bool(v))
    }

    /// The number of the object `i` places under the top.
    pub(super) fn num_at(&self, i: usize) -> R<f64> {
        num(self.top(i)?)
    }

    pub(super) fn int_at(&self, i: usize) -> R<i64> {
        int(self.top(i)?)
    }

    /// Pops `N` numbers (the deepest first). Nothing is popped if one is
    /// not a number.
    pub(super) fn nums<const N: usize>(&mut self) -> R<[f64; N]> {
        let mut v = [0.0; N];
        for (k, slot) in v.iter_mut().enumerate() {
            *slot = self.num_at(N - 1 - k)?;
        }
        self.drop_n(N);
        Ok(v)
    }

    /// Counts the operands down to the top mark.
    pub(super) fn count_to_mark(&self) -> R<usize> {
        self.ops
            .iter()
            .rev()
            .position(|o| matches!(o, Obj::Mark))
            .ok_or(PsError::UnmatchedMark)
    }

    // ----- Running -----

    pub(super) fn push_frame(&mut self, f: Frame) -> R {
        if self.exec.len() >= MAX_FRAMES {
            return Err(PsError::LimitCheck);
        }
        self.exec.push(f);
        Ok(())
    }

    /// Runs an object as `exec` does.
    pub(super) fn execute(&mut self, o: Obj) -> R {
        match o {
            Obj::Arr(a, true) => {
                if a.len > 0 {
                    self.push_frame(Frame::Proc { arr: a, pos: 0 })?;
                }
                Ok(())
            }
            Obj::Name(n, true) => self.execute_name(n),
            Obj::Op(k) => self.call(k),
            Obj::Str(s, true) => {
                let bytes = self.sbytes(s).to_vec();
                self.run_text(bytes)
            }
            Obj::File(0, true) => Ok(()),
            Obj::File(f, true) => {
                super::ops::materialize(self, f, 0)?;
                let file = &mut self.files[f as usize];
                let bytes = file.data.get(file.pos..).unwrap_or_default().to_vec();
                file.pos = file.data.len();
                self.run_text(bytes)
            }
            other => self.push(other),
        }
    }

    /// Runs `bytes` as the text of a program.
    fn run_text(&mut self, bytes: Vec<u8>) -> R {
        self.charge(bytes.len())?;
        let src = self.sources.len() as u32;
        self.sources.push(Rc::from(bytes));
        self.push_frame(Frame::Tokens { src, pos: 0 })
    }

    fn execute_name(&mut self, n: u32) -> R {
        let mut v = self.lookup(n).ok_or(PsError::Undefined(n))?;
        // A name can have an executable name as its value: follow a few.
        for _ in 0..16 {
            match v {
                Obj::Name(m, true) => v = self.lookup(m).ok_or(PsError::Undefined(m))?,
                _ => return self.execute(v),
            }
        }
        Err(PsError::LimitCheck)
    }

    pub(super) fn call(&mut self, k: u16) -> R {
        let f = self.op_fns[k as usize];
        f(self)
    }

    /// An object that the machine finds in a file or a procedure.
    fn direct(&mut self, o: Obj) -> R {
        match o {
            Obj::Arr(_, true) => self.push(o),
            Obj::Name(n, true) => self.execute_name(n),
            Obj::Op(k) => self.call(k),
            other => self.push(other),
        }
    }

    /// Runs until the program ends or a limit stops it.
    pub(super) fn run(&mut self) -> R {
        while !self.exec.is_empty() {
            self.steps += 1;
            if self.steps > self.limits.steps {
                return Err(PsError::TooManySteps);
            }
            if let Err(e) = self.step() {
                if e.stops_the_run() {
                    return Err(e);
                }
                self.on_error(e)?;
            }
        }
        Ok(())
    }

    fn step(&mut self) -> R {
        let n = self.exec.len() - 1;
        match &mut self.exec[n] {
            Frame::Tokens { src, pos } => {
                let (src, mut p) = (*src, *pos);
                let tok = self.scan(src, &mut p);
                self.set_text_pos(n, p);
                match tok? {
                    Some(o) => self.direct(o),
                    None => {
                        self.exec.truncate(n);
                        Ok(())
                    }
                }
            }
            Frame::Proc { arr, pos } => {
                let (a, p) = (*arr, *pos);
                if p >= a.len {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *pos += 1;
                if p + 1 >= a.len {
                    // The last object: the frame goes first (tail calls).
                    self.exec.truncate(n);
                }
                let o = self.arrays[a.id as usize][(a.start + p) as usize];
                self.direct(o)
            }
            Frame::Loop { proc } => {
                let p = *proc;
                self.execute(p)
            }
            Frame::Repeat { proc, left } => {
                if *left <= 0 {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *left -= 1;
                let p = *proc;
                self.execute(p)
            }
            Frame::ForInt { proc, cur, inc, lim } => {
                let (c, i, l, p) = (*cur, *inc, *lim, *proc);
                if (i >= 0 && c > l) || (i < 0 && c < l) {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *cur = c.saturating_add(i);
                self.push_int(c)?;
                self.execute(p)
            }
            Frame::ForReal { proc, cur, inc, lim } => {
                let (c, i, l, p) = (*cur, *inc, *lim, *proc);
                if (i >= 0.0 && c > l) || (i < 0.0 && c < l) {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *cur = c + i;
                self.push_real(c)?;
                self.execute(p)
            }
            Frame::ForallArr { proc, arr, i } => {
                let (p, a, k) = (*proc, *arr, *i);
                if k >= a.len {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *i += 1;
                let o = self.arrays[a.id as usize][(a.start + k) as usize];
                self.push(o)?;
                self.execute(p)
            }
            Frame::ForallStr { proc, s, i } => {
                let (p, s, k) = (*proc, *s, *i);
                if k >= s.len {
                    self.exec.truncate(n);
                    return Ok(());
                }
                *i += 1;
                let b = self.strings[s.id as usize][(s.start + k) as usize];
                self.push(Obj::Int(i32::from(b)))?;
                self.execute(p)
            }
            Frame::ForallDict { proc, dict, i } => {
                let (p, d, k) = (*proc, *dict, *i);
                let Some(&(key, val)) = self.dicts[d as usize].entries.get(k) else {
                    self.exec.truncate(n);
                    return Ok(());
                };
                if let Some(Frame::ForallDict { i, .. }) = self.exec.get_mut(n) {
                    *i += 1;
                }
                self.push(key)?;
                self.push(val)?;
                self.execute(p)
            }
            Frame::Stopped => {
                self.exec.truncate(n);
                self.push_bool(false)
            }
            Frame::Then(_) => match self.exec.pop() {
                Some(Frame::Then(c)) => self.resume(c),
                _ => Ok(()),
            },
        }
    }

    /// Goes on with the work of an operator after its procedure.
    fn resume(&mut self, c: Cont) -> R {
        match c {
            Cont::GRestore => {
                super::graphics::grestore(self);
                Ok(())
            }
            Cont::Tint { n, alt } => {
                let mut v = vec![0.0; n];
                for (k, slot) in v.iter_mut().enumerate() {
                    *slot = self.num_at(n - 1 - k).unwrap_or(0.0);
                }
                self.drop_n(n);
                super::graphics::set_paint_from(self, alt, &v);
                Ok(())
            }
            Cont::Picture {
                procs,
                next,
                mut left,
                first,
            } => {
                if !first {
                    match self.pop()? {
                        Obj::Str(s, _) if s.len > 0 => left -= i64::from(s.len),
                        _ => return Ok(()),
                    }
                }
                if left <= 0 || procs.is_empty() {
                    return Ok(());
                }
                let p = procs[next % procs.len()];
                self.push_frame(Frame::Then(Cont::Picture {
                    next: next + 1,
                    procs,
                    left,
                    first: false,
                }))?;
                self.execute(p)
            }
        }
    }

    /// An error: `stopped` takes it, else the run goes on after the object
    /// of the file that caused it.
    fn on_error(&mut self, e: PsError) -> R {
        if let Some(i) = self.exec.iter().rposition(|f| matches!(f, Frame::Stopped)) {
            self.exec.truncate(i);
            self.note_error(&e);
            return self.push_bool(true);
        }
        if self.ai_script
            && let PsError::Undefined(n) = e
        {
            // An operator of the script that the reader does not use. A
            // paint operator of a guide (`*` and a letter) ends the path.
            if self.name_bytes(n).first() == Some(&b'*') {
                super::graphics::new_path(self);
            }
            self.ops.clear();
            self.unwind_to_text();
            return Ok(());
        }
        self.errors += 1;
        if self.first_error.is_none() {
            let what = match &e {
                // A name from damaged data can be long: its start only.
                PsError::Undefined(n) => {
                    let name = self.name_text(*n);
                    match name.char_indices().nth(40) {
                        Some((cut, _)) => format!("undefined {}…", &name[..cut]),
                        None => format!("undefined {name}"),
                    }
                }
                other => other.name().to_string(),
            };
            self.first_error = Some(what);
        }
        if self.errors > MAX_ERRORS {
            return Err(PsError::TooManyErrors);
        }
        self.unwind_to_text();
        Ok(())
    }

    fn note_error(&mut self, e: &PsError) {
        let d = self.dollar_error;
        let newerror = self.name_obj("newerror");
        let errorname = self.name_obj("errorname");
        let value = self.name_obj(e.name());
        let _ = self.dict_put(d, newerror, Obj::Bool(true));
        let _ = self.dict_put(d, errorname, value);
    }

    /// Removes the frames down to the text that runs.
    fn unwind_to_text(&mut self) {
        while let Some(f) = self.exec.last() {
            if matches!(f, Frame::Tokens { .. }) {
                break;
            }
            self.exec.pop();
        }
    }

    /// `exit`: ends the loop that runs.
    pub(super) fn exit_loop(&mut self) -> R {
        while let Some(f) = self.exec.last() {
            match f {
                Frame::Loop { .. }
                | Frame::Repeat { .. }
                | Frame::ForInt { .. }
                | Frame::ForReal { .. }
                | Frame::ForallArr { .. }
                | Frame::ForallStr { .. }
                | Frame::ForallDict { .. } => {
                    self.exec.pop();
                    return Ok(());
                }
                Frame::Stopped | Frame::Tokens { .. } => return Err(PsError::InvalidExit),
                _ => {
                    self.exec.pop();
                }
            }
        }
        Err(PsError::InvalidExit)
    }

    /// `stop`: ends the `stopped` that runs.
    pub(super) fn stop(&mut self) -> R {
        match self.exec.iter().rposition(|f| matches!(f, Frame::Stopped)) {
            Some(i) => {
                self.exec.truncate(i);
                self.push_bool(true)
            }
            None => {
                self.unwind_to_text();
                Ok(())
            }
        }
    }

    // ----- The text of the file that runs -----

    /// The text frame that runs now (for `currentfile`), its bytes and its position.
    pub(super) fn text_rest(&self) -> Option<(usize, Rc<[u8]>, usize)> {
        let i = self.exec.iter().rposition(|f| matches!(f, Frame::Tokens { .. }))?;
        let Frame::Tokens { src, pos } = self.exec[i] else {
            return None;
        };
        Some((i, self.sources[src as usize].clone(), pos))
    }

    pub(super) fn set_text_pos(&mut self, frame: usize, p: usize) {
        if let Some(Frame::Tokens { pos, .. }) = self.exec.get_mut(frame) {
            *pos = p;
        }
    }
}

/// The number of a numeric object.
pub(super) fn num(o: Obj) -> R<f64> {
    match o {
        Obj::Int(i) => Ok(f64::from(i)),
        Obj::Real(r) => Ok(r),
        _ => Err(PsError::TypeCheck),
    }
}

/// The integer of an object (also a real with no fraction).
pub(super) fn int(o: Obj) -> R<i64> {
    match o {
        Obj::Int(i) => Ok(i64::from(i)),
        Obj::Real(r) if r.fract() == 0.0 && r.abs() < 9e15 => Ok(r as i64),
        Obj::Real(_) => Err(PsError::RangeCheck),
        _ => Err(PsError::TypeCheck),
    }
}
