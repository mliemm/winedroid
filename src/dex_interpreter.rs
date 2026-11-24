use std::fmt;

use crate::dex::DexCodeItem;
use crate::error::{Result, WinedroidError};
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq)]
pub enum DexExecutionResult {
    Void,
    Int(i32),
    Value(DexValue),
    Throw(DexValue),
    Parked(DexParkedExecution),
}

impl fmt::Display for DexExecutionResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => write!(formatter, "void"),
            Self::Int(value) => write!(formatter, "int {value}"),
            Self::Value(value) => write!(formatter, "{value}"),
            Self::Throw(value) => write!(formatter, "throw {value}"),
            Self::Parked(_) => write!(formatter, "parked"),
        }
    }
}

pub type DexStr = std::rc::Rc<str>;

#[derive(Debug, Clone, PartialEq)]
pub enum DexValue {
    Uninitialized,
    Null,
    Int(i32),
    Long(i64),
    Float(f32),
    String(DexStr),
    Object(DexObjectRef),
    Reference(DexStr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexObjectRef {
    pub id: u64,
    pub class_descriptor: DexStr,
}

pub type DexInvocationRegisters = SmallVec<[u16; 5]>;
pub type DexInvocationArguments = SmallVec<[DexValue; 5]>;

impl fmt::Display for DexValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => write!(formatter, "uninitialized"),
            Self::Null => write!(formatter, "null"),
            Self::Int(value) => write!(formatter, "int {value}"),
            Self::Long(value) => write!(formatter, "long {value}"),
            Self::Float(value) => write!(formatter, "float {value}"),
            Self::String(value) => write!(formatter, "string {value:?}"),
            Self::Object(value) => {
                write!(formatter, "object #{} {}", value.id, value.class_descriptor)
            }
            Self::Reference(value) => write!(formatter, "ref {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexInvokeKind {
    Virtual,
    Super,
    Direct,
    Static,
    Interface,
}

impl fmt::Display for DexInvokeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Virtual => write!(formatter, "invoke-virtual"),
            Self::Super => write!(formatter, "invoke-super"),
            Self::Direct => write!(formatter, "invoke-direct"),
            Self::Static => write!(formatter, "invoke-static"),
            Self::Interface => write!(formatter, "invoke-interface"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexInvocation {
    pub kind: DexInvokeKind,
    pub range: bool,
    pub method_index: u32,
    pub argument_registers: DexInvocationRegisters,
    pub arguments: DexInvocationArguments,
}

pub struct DexInvocationFrame<'a> {
    kind: DexInvokeKind,
    range: bool,
    method_index: u32,
    argument_registers: DexInvocationRegisters,
    frame: &'a DexFrame,
}

impl DexInvocationFrame<'_> {
    pub fn kind(&self) -> DexInvokeKind {
        self.kind
    }

    pub fn method_index(&self) -> u32 {
        self.method_index
    }

    pub fn argument_count(&self) -> usize {
        self.argument_registers.len()
    }

    pub fn object_argument_class_descriptor(&self, argument_index: usize) -> Result<Option<&str>> {
        let Some(register) = self.argument_registers.get(argument_index).copied() else {
            return Ok(None);
        };

        self.frame.object_class_descriptor(register)
    }

    pub fn argument_class_descriptor(&self, argument_index: usize) -> Result<Option<&str>> {
        let Some(register) = self.argument_registers.get(argument_index).copied() else {
            return Ok(None);
        };

        self.frame.value_class_descriptor(register)
    }

    pub fn read_argument(&self, argument_index: usize) -> Result<Option<DexValue>> {
        let Some(register) = self.argument_registers.get(argument_index).copied() else {
            return Ok(None);
        };

        self.frame.read_value(register).map(Some)
    }

    pub(crate) fn copy_argument_to_frame(
        &self,
        argument_index: usize,
        destination: &mut DexFrame,
        destination_register: u16,
    ) -> Result<()> {
        let Some(source_register) = self.argument_registers.get(argument_index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX invocation has no argument at index {argument_index}"
            )));
        };

        destination.copy_register_from_frame(destination_register, self.frame, source_register)
    }

    pub fn materialize(&self) -> Result<DexInvocation> {
        let mut arguments = DexInvocationArguments::with_capacity(self.argument_registers.len());
        for register in &self.argument_registers {
            arguments.push(self.frame.read_value(*register)?);
        }

        Ok(DexInvocation {
            kind: self.kind,
            range: self.range,
            method_index: self.method_index,
            argument_registers: self.argument_registers.clone(),
            arguments,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexFieldAccess {
    pub field_index: u32,
    pub destination_register: u16,
    pub object_register: u16,
    pub object: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexFieldWrite {
    pub field_index: u32,
    pub value_register: u16,
    pub object_register: u16,
    pub value: DexValue,
    pub object: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexStaticFieldAccess {
    pub field_index: u32,
    pub destination_register: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexStaticFieldWrite {
    pub field_index: u32,
    pub value_register: u16,
    pub value: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexStaticFieldReadResult {
    Value(DexValue),
    Throw(DexValue),
    Parked(DexParkedExecution),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexStaticFieldWriteResult {
    Complete,
    Throw(DexValue),
    Parked(DexParkedExecution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexStringAccess {
    pub string_index: u32,
    pub destination_register: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexNewInstance {
    pub type_index: u32,
    pub destination_register: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexClassAccess {
    pub type_index: u32,
    pub destination_register: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexNewArray {
    pub type_index: u32,
    pub destination_register: u16,
    pub size_register: u16,
    pub size: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexArrayLength {
    pub destination_register: u16,
    pub array_register: u16,
    pub array: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexArrayRead {
    pub destination_register: u16,
    pub array_register: u16,
    pub index_register: u16,
    pub array: DexValue,
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexArrayWrite {
    pub value_register: u16,
    pub array_register: u16,
    pub index_register: u16,
    pub value: DexValue,
    pub array: DexValue,
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexFilledNewArray {
    pub type_index: u32,
    pub first_register: u16,
    pub argument_registers: Vec<u16>,
    pub values: Vec<DexValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexTypeCheck {
    pub type_index: u32,
    pub register: u16,
    pub value: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexInstanceOf {
    pub type_index: u32,
    pub destination_register: u16,
    pub value_register: u16,
    pub value: DexValue,
}

impl DexInvocation {
    pub fn instruction_name(&self) -> String {
        if self.range {
            return format!("{}/range", self.kind);
        }

        self.kind.to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexInvocationResult {
    Void,
    Value(DexValue),
    RawValue(DexPendingResult),
    Throw(DexValue),
    Parked(DexParkedExecution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DexGeneratedException {
    pub class_descriptor: String,
    pub detail_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexNewInstanceResult {
    Allocated(DexValue),
    Throw(DexGeneratedException),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DexCheckCastResult {
    Success,
    Throw(DexGeneratedException),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexParkedExecution {
    steps: Vec<DexParkedStep>,
    park_timeout_nanos: Option<i64>,
    waiting_class_descriptor: Option<String>,
}

impl DexParkedExecution {
    pub fn empty() -> Self {
        Self {
            steps: Vec::new(),
            park_timeout_nanos: None,
            waiting_class_descriptor: None,
        }
    }

    pub fn waiting_for_class(class_descriptor: String) -> Self {
        Self {
            steps: Vec::new(),
            park_timeout_nanos: None,
            waiting_class_descriptor: Some(class_descriptor),
        }
    }

    // cite: libcore LockSupport.java#parkNanos
    pub fn with_park_timeout(nanos: i64) -> Self {
        let mut execution = Self::empty();
        execution.park_timeout_nanos = Some(nanos);
        execution
    }

    pub fn park_timeout_nanos(&self) -> Option<i64> {
        self.park_timeout_nanos
    }

    pub fn waiting_class_descriptor(&self) -> Option<&str> {
        self.waiting_class_descriptor.as_deref()
    }

    pub fn pop_next_step(&mut self) -> Option<DexParkedStep> {
        if self.steps.is_empty() {
            return None;
        }

        Some(self.steps.remove(0))
    }

    pub fn replace_void_result_after_current_steps(&mut self, value: DexValue) {
        self.steps.push(DexParkedStep::ReplaceVoidWithValue(value));
    }

    pub fn push_runtime_action(&mut self, action_id: u64) {
        self.steps.push(DexParkedStep::RuntimeAction(action_id));
    }

    pub fn runtime_action_ids(&self) -> Vec<u64> {
        self.steps
            .iter()
            .filter_map(|step| match step {
                DexParkedStep::RuntimeAction(action_id) => Some(*action_id),
                DexParkedStep::Frame(_) | DexParkedStep::ReplaceVoidWithValue(_) => None,
            })
            .collect()
    }

    pub fn append_remaining_steps(&mut self, mut execution: DexParkedExecution) {
        self.steps.append(&mut execution.steps);
    }

    fn push_caller(&mut self, frame: DexParkedFrame) {
        self.steps.push(DexParkedStep::Frame(frame));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexParkedStep {
    Frame(DexParkedFrame),
    ReplaceVoidWithValue(DexValue),
    RuntimeAction(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexParkedFrame {
    dex_file_index: usize,
    current_class_descriptor: Option<String>,
    code: DexCodeItem,
    frame: DexFrame,
    pending_result: Option<DexPendingResult>,
    pending_exception: Option<DexValue>,
    pc: usize,
    resume_kind: DexParkedFrameResumeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DexParkedFrameResumeKind {
    Invocation,
    RetryInstruction,
}

impl DexParkedFrame {
    pub fn dex_file_index(&self) -> usize {
        self.dex_file_index
    }

    pub fn current_class_descriptor(&self) -> Option<&str> {
        self.current_class_descriptor.as_deref()
    }

    fn new(
        dex_file_index: usize,
        current_class_descriptor: Option<String>,
        code: &DexCodeItem,
        frame: &DexFrame,
        pending_result: &Option<DexPendingResult>,
        pending_exception: &Option<DexValue>,
        pc: usize,
    ) -> Self {
        Self {
            dex_file_index,
            current_class_descriptor,
            code: code.clone(),
            frame: frame.clone(),
            pending_result: pending_result.clone(),
            pending_exception: pending_exception.clone(),
            pc,
            resume_kind: DexParkedFrameResumeKind::Invocation,
        }
    }

    fn retry_instruction(
        dex_file_index: usize,
        current_class_descriptor: Option<String>,
        code: &DexCodeItem,
        frame: &DexFrame,
        pending_result: &Option<DexPendingResult>,
        pending_exception: &Option<DexValue>,
        pc: usize,
    ) -> Self {
        Self {
            dex_file_index,
            current_class_descriptor,
            code: code.clone(),
            frame: frame.clone(),
            pending_result: pending_result.clone(),
            pending_exception: pending_exception.clone(),
            pc,
            resume_kind: DexParkedFrameResumeKind::RetryInstruction,
        }
    }
}

pub trait DexInvocationHandler {
    fn invoke(&mut self, invocation: DexInvocation) -> Result<DexInvocationResult>;

    fn current_class_descriptor(&self) -> Option<&str> {
        None
    }

    fn invoke_from_frame(
        &mut self,
        invocation: DexInvocationFrame<'_>,
    ) -> Result<DexInvocationResult> {
        self.invoke(invocation.materialize()?)
    }

    fn exception_matches(&mut self, exception: &DexValue, type_descriptor: &str) -> Result<bool> {
        let DexValue::Object(object) = exception else {
            return Ok(false);
        };

        Ok(object.class_descriptor == std::rc::Rc::from(type_descriptor))
    }

    fn allocate_generated_exception(
        &mut self,
        exception: DexGeneratedException,
    ) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX generated exception allocation not implemented\nclass: {}",
            exception.class_descriptor
        )))
    }

    fn read_instance_field(&mut self, access: DexFieldAccess) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX instance field read not implemented\nfield index: {}\ndestination register: v{}\nobject register: v{}\nnext missing primitive: object field storage",
            access.field_index, access.destination_register, access.object_register
        )))
    }

    fn write_instance_field(&mut self, write: DexFieldWrite) -> Result<()> {
        Err(WinedroidError::new(format!(
            "DEX instance field write not implemented\nfield index: {}\nvalue register: v{}\nobject register: v{}\nnext missing primitive: object field storage",
            write.field_index, write.value_register, write.object_register
        )))
    }

    fn read_static_field(
        &mut self,
        access: DexStaticFieldAccess,
    ) -> Result<DexStaticFieldReadResult> {
        Err(WinedroidError::new(format!(
            "DEX static field read not implemented\nfield index: {}\ndestination register: v{}\nnext missing primitive: static field storage",
            access.field_index, access.destination_register
        )))
    }

    fn write_static_field(
        &mut self,
        write: DexStaticFieldWrite,
    ) -> Result<DexStaticFieldWriteResult> {
        Err(WinedroidError::new(format!(
            "DEX static field write not implemented\nfield index: {}\nvalue register: v{}\nnext missing primitive: static field storage",
            write.field_index, write.value_register
        )))
    }

    fn load_string(&mut self, access: DexStringAccess) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX string constant load not implemented\nstring index: {}\ndestination register: v{}\nnext missing primitive: DEX string table resolution",
            access.string_index, access.destination_register
        )))
    }

    fn load_class(&mut self, access: DexClassAccess) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX class constant load not implemented\ntype index: {}\ndestination register: v{}\nnext missing primitive: DEX type table resolution",
            access.type_index, access.destination_register
        )))
    }

    fn allocate_object(&mut self, instance: DexNewInstance) -> Result<DexNewInstanceResult> {
        Err(WinedroidError::new(format!(
            "DEX object allocation not implemented\ntype index: {}\ndestination register: v{}\nnext missing primitive: object allocation",
            instance.type_index, instance.destination_register
        )))
    }

    fn allocate_array(&mut self, array: DexNewArray) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX array allocation not implemented\ntype index: {}\ndestination register: v{}\nsize register: v{}\nnext missing primitive: array allocation",
            array.type_index, array.destination_register, array.size_register
        )))
    }

    fn array_length(&mut self, length: DexArrayLength) -> Result<i32> {
        Err(WinedroidError::new(format!(
            "DEX array-length not implemented\ndestination register: v{}\narray register: v{}\narray: {}\nnext missing primitive: array length storage",
            length.destination_register, length.array_register, length.array
        )))
    }

    fn read_array(&mut self, read: DexArrayRead) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX array read not implemented\ndestination register: v{}\narray register: v{}\nindex register: v{}\nindex: {}\narray: {}\nnext missing primitive: array value storage",
            read.destination_register,
            read.array_register,
            read.index_register,
            read.index,
            read.array
        )))
    }

    fn write_array(&mut self, write: DexArrayWrite) -> Result<()> {
        Err(WinedroidError::new(format!(
            "DEX array write not implemented\nvalue register: v{}\narray register: v{}\nindex register: v{}\nindex: {}\narray: {}\nvalue: {}\nnext missing primitive: array value storage",
            write.value_register,
            write.array_register,
            write.index_register,
            write.index,
            write.array,
            write.value
        )))
    }

    fn allocate_filled_array(&mut self, array: DexFilledNewArray) -> Result<DexValue> {
        Err(WinedroidError::new(format!(
            "DEX filled array allocation not implemented\ntype index: {}\nfirst register: v{}\nvalue count: {}\nnext missing primitive: filled array allocation",
            array.type_index,
            array.first_register,
            array.values.len()
        )))
    }

    fn check_cast(&mut self, check: DexTypeCheck) -> Result<DexCheckCastResult> {
        Err(WinedroidError::new(format!(
            "DEX check-cast not implemented\ntype index: {}\nregister: v{}\nnext missing primitive: class hierarchy checks",
            check.type_index, check.register
        )))
    }

    fn instance_of(&mut self, check: DexInstanceOf) -> Result<i32> {
        Err(WinedroidError::new(format!(
            "DEX instance-of not implemented\ntype index: {}\ndestination register: v{}\nvalue register: v{}\nnext missing primitive: class hierarchy checks",
            check.type_index, check.destination_register, check.value_register
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RegTag {
    #[default]
    Uninitialized,
    Null,
    Int,
    Long,
    Float,
    String,
    Reference,
    Object,
}

// cite: art shadow_frame.h#GetVReg, SetVReg, GetVRegReference, SetVRegReference, vregs_
#[derive(Debug, Clone, PartialEq)]
struct DexRawValue {
    tag: RegTag,
    word: u64,
    boxed: Option<DexStr>,
}

impl DexRawValue {
    fn from_value(value: DexValue) -> Self {
        let (tag, word, boxed) = match value {
            DexValue::Uninitialized => (RegTag::Uninitialized, 0, None),
            DexValue::Null => (RegTag::Null, 0, None),
            DexValue::Int(value) => (RegTag::Int, u64::from(value as u32), None),
            DexValue::Long(value) => (RegTag::Long, value as u64, None),
            DexValue::Float(value) => (RegTag::Float, u64::from(value.to_bits()), None),
            DexValue::String(value) => (RegTag::String, 0, Some(value)),
            DexValue::Reference(value) => (RegTag::Reference, 0, Some(value)),
            DexValue::Object(value) => (RegTag::Object, value.id, Some(value.class_descriptor)),
        };

        Self { tag, word, boxed }
    }

    fn into_value(self) -> Result<DexValue> {
        match self.tag {
            RegTag::Uninitialized => Err(WinedroidError::new("DEX raw value is uninitialized")),
            RegTag::Null => Ok(DexValue::Null),
            RegTag::Int => Ok(DexValue::Int(self.word as u32 as i32)),
            RegTag::Long => Ok(DexValue::Long(self.word as i64)),
            RegTag::Float => Ok(DexValue::Float(f32::from_bits(self.word as u32))),
            RegTag::String => Ok(DexValue::String(self.boxed.ok_or_else(|| {
                WinedroidError::new("DEX raw value lost its string payload")
            })?)),
            RegTag::Reference => Ok(DexValue::Reference(self.boxed.ok_or_else(|| {
                WinedroidError::new("DEX raw value lost its reference payload")
            })?)),
            RegTag::Object => Ok(DexValue::Object(DexObjectRef {
                id: self.word,
                class_descriptor: self
                    .boxed
                    .ok_or_else(|| WinedroidError::new("DEX raw value lost its object class"))?,
            })),
        }
    }

    fn write_to_frame(self, frame: &mut DexFrame, register: u16) -> Result<()> {
        frame.set_raw(register, self)
    }

    fn write_wide_to_frame(self, frame: &mut DexFrame, register: u16) -> Result<()> {
        if self.tag != RegTag::Long {
            return Err(WinedroidError::new(format!(
                "move-result-wide expected a wide pending result, found {:?}",
                self.tag
            )));
        }

        frame.set_raw(register, self.clone())?;
        frame.set_raw(register + 1, self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DexPendingResult {
    value: DexRawValue,
}

impl DexPendingResult {
    pub(crate) fn from_value(value: DexValue) -> Self {
        Self {
            value: DexRawValue::from_value(value),
        }
    }

    pub(crate) fn into_value(self) -> Result<DexValue> {
        self.value.into_value()
    }

    fn write_to_frame(self, frame: &mut DexFrame, register: u16) -> Result<()> {
        self.value.write_to_frame(frame, register)
    }

    fn write_wide_to_frame(self, frame: &mut DexFrame, register: u16) -> Result<()> {
        self.value.write_wide_to_frame(frame, register)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DexFrame {
    tags: Vec<RegTag>,
    words: Vec<u64>,
    boxed: Vec<Option<DexStr>>,
}

type FramePoolParts = (Vec<RegTag>, Vec<u64>, Vec<Option<DexStr>>);

thread_local! {
    static FRAME_POOL: std::cell::RefCell<Vec<FramePoolParts>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

const FRAME_POOL_LIMIT: usize = 64;

impl Drop for DexFrame {
    fn drop(&mut self) {
        let mut tags = std::mem::take(&mut self.tags);
        let words = std::mem::take(&mut self.words);
        let mut boxed = std::mem::take(&mut self.boxed);
        tags.clear();
        boxed.clear();
        let _ = FRAME_POOL.try_with(|pool| {
            if let Ok(mut pool) = pool.try_borrow_mut() {
                if pool.len() < FRAME_POOL_LIMIT {
                    pool.push((tags, words, boxed));
                }
            }
        });
    }
}

impl DexFrame {
    pub(crate) fn new(register_count: u16) -> Self {
        let count = register_count as usize;
        let (mut tags, mut words, mut boxed) = FRAME_POOL
            .with(|pool| pool.borrow_mut().pop())
            .unwrap_or_default();
        tags.clear();
        words.clear();
        boxed.clear();
        tags.resize(count, RegTag::Uninitialized);
        words.resize(count, 0);
        boxed.resize(count, None);
        DexFrame { tags, words, boxed }
    }

    pub(crate) fn new_zeroed(register_count: u16) -> Self {
        let mut frame = Self::new(register_count);
        for register in 0..register_count {
            let _ = frame.set_int(register, 0);
        }
        frame
    }

    pub(crate) fn from_values(values: Vec<DexValue>) -> Self {
        let mut frame = DexFrame::new(values.len() as u16);
        for (index, value) in values.into_iter().enumerate() {
            let _ = frame.set_register(index as u16, value);
        }
        frame
    }

    #[cfg(test)]
    pub(crate) fn from_zeroed_registers_and_arguments(
        register_count: u16,
        ins_size: u16,
        initial_arguments: Option<&[DexValue]>,
    ) -> Result<Self> {
        let mut frame = Self::new_zeroed(register_count);
        let Some(arguments) = initial_arguments else {
            return Ok(frame);
        };
        if arguments.len() != ins_size as usize {
            return Err(WinedroidError::new(format!(
                "DEX method expects {ins_size} input register(s), got {} argument value(s)",
                arguments.len()
            )));
        }
        if ins_size > register_count {
            return Err(WinedroidError::new(format!(
                "DEX code item has {ins_size} input register(s) but only {register_count} total register(s)"
            )));
        }
        let first_parameter = register_count - ins_size;
        for (index, value) in arguments.iter().enumerate() {
            frame.set_register_from_value(first_parameter + index as u16, value)?;
        }
        Ok(frame)
    }

    pub(crate) fn read_values_window(&self, base: usize, count: usize) -> Result<Vec<DexValue>> {
        if base >= self.tags.len() {
            return Ok(Vec::new());
        }
        let end = base.saturating_add(count).min(self.tags.len());
        let mut values = Vec::with_capacity(end - base);
        for index in base..end {
            values.push(self.read_value(index as u16)?);
        }
        Ok(values)
    }

    pub(crate) fn read_all_values(&self) -> Result<Vec<DexValue>> {
        self.read_values_window(0, self.tags.len())
    }

    pub(crate) fn set_register(&mut self, register: u16, value: DexValue) -> Result<()> {
        self.set_raw(register, DexRawValue::from_value(value))
    }

    fn set_raw(&mut self, register: u16, value: DexRawValue) -> Result<()> {
        let index = register as usize;
        if index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        }
        self.tags[index] = value.tag;
        self.words[index] = value.word;
        self.boxed[index] = value.boxed;
        Ok(())
    }

    pub(crate) fn set_register_from_value(
        &mut self,
        register: u16,
        value: &DexValue,
    ) -> Result<()> {
        match value {
            DexValue::Uninitialized => self.set_register(register, DexValue::Uninitialized),
            DexValue::Null => self.set_register(register, DexValue::Null),
            DexValue::Int(value) => self.set_int(register, *value),
            DexValue::Long(value) => self.set_long(register, *value),
            DexValue::Float(value) => self.set_float(register, *value),
            DexValue::String(value) => self.set_register(register, DexValue::String(value.clone())),
            DexValue::Reference(value) => {
                self.set_register(register, DexValue::Reference(value.clone()))
            }
            DexValue::Object(value) => self.set_register(register, DexValue::Object(value.clone())),
        }
    }

    #[inline]
    pub(crate) fn set_int(&mut self, register: u16, value: i32) -> Result<()> {
        let index = register as usize;
        if index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        }
        self.tags[index] = RegTag::Int;
        self.words[index] = u64::from(value as u32);
        if self.boxed[index].is_some() {
            self.boxed[index] = None;
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn set_long(&mut self, register: u16, value: i64) -> Result<()> {
        let index = register as usize;
        if index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        }
        self.tags[index] = RegTag::Long;
        self.words[index] = value as u64;
        if self.boxed[index].is_some() {
            self.boxed[index] = None;
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn set_float(&mut self, register: u16, value: f32) -> Result<()> {
        let index = register as usize;
        if index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        }
        self.tags[index] = RegTag::Float;
        self.words[index] = u64::from(value.to_bits());
        if self.boxed[index].is_some() {
            self.boxed[index] = None;
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn set_long_pair(&mut self, register: u16, value: i64) -> Result<()> {
        self.set_long(register, value)?;
        self.set_long(register + 1, value)
    }

    #[inline]
    pub(crate) fn copy_register(&mut self, destination: u16, source: u16) -> Result<()> {
        let source_index = source as usize;
        let Some(tag) = self.tags.get(source_index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX register v{source} is outside the frame"
            )));
        };
        if tag == RegTag::Uninitialized {
            return Err(WinedroidError::new(format!(
                "DEX register v{source} is uninitialized"
            )));
        }
        let destination_index = destination as usize;
        if destination_index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{destination} is outside the frame"
            )));
        }
        self.tags[destination_index] = tag;
        self.words[destination_index] = self.words[source_index];
        self.boxed[destination_index] = self.boxed[source_index].clone();
        Ok(())
    }

    pub(crate) fn copy_register_from_frame(
        &mut self,
        destination: u16,
        source_frame: &DexFrame,
        source: u16,
    ) -> Result<()> {
        let source_index = source as usize;
        let Some(tag) = source_frame.tags.get(source_index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX register v{source} is outside the frame"
            )));
        };
        if tag == RegTag::Uninitialized {
            return Err(WinedroidError::new(format!(
                "DEX register v{source} is uninitialized"
            )));
        }
        let destination_index = destination as usize;
        if destination_index >= self.tags.len() {
            return Err(WinedroidError::new(format!(
                "DEX register v{destination} is outside the frame"
            )));
        }
        self.tags[destination_index] = tag;
        self.words[destination_index] = source_frame.words[source_index];
        self.boxed[destination_index] = source_frame.boxed[source_index].clone();
        Ok(())
    }

    pub(crate) fn read_value(&self, register: u16) -> Result<DexValue> {
        let index = register as usize;
        let Some(tag) = self.tags.get(index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        };
        let word = self.words[index];
        match tag {
            RegTag::Uninitialized => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            RegTag::Null => Ok(DexValue::Null),
            RegTag::Int => Ok(DexValue::Int(word as u32 as i32)),
            RegTag::Long => Ok(DexValue::Long(word as i64)),
            RegTag::Float => Ok(DexValue::Float(f32::from_bits(word as u32))),
            RegTag::String => Ok(DexValue::String(self.boxed[index].clone().ok_or_else(
                || WinedroidError::new(format!("DEX register v{register} lost its string payload")),
            )?)),
            RegTag::Reference => Ok(DexValue::Reference(self.boxed[index].clone().ok_or_else(
                || {
                    WinedroidError::new(format!(
                        "DEX register v{register} lost its reference payload"
                    ))
                },
            )?)),
            RegTag::Object => Ok(DexValue::Object(DexObjectRef {
                id: word,
                class_descriptor: self.boxed[index].clone().ok_or_else(|| {
                    WinedroidError::new(format!("DEX register v{register} lost its object class"))
                })?,
            })),
        }
    }

    pub(crate) fn object_class_descriptor(&self, register: u16) -> Result<Option<&str>> {
        let index = register as usize;
        let Some(tag) = self.tags.get(index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        };

        match tag {
            RegTag::Object => Ok(Some(self.boxed[index].as_deref().ok_or_else(|| {
                WinedroidError::new(format!("DEX register v{register} lost its object class"))
            })?)),
            RegTag::Uninitialized => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            RegTag::Null
            | RegTag::Int
            | RegTag::Long
            | RegTag::Float
            | RegTag::String
            | RegTag::Reference => Ok(None),
        }
    }

    pub(crate) fn value_class_descriptor(&self, register: u16) -> Result<Option<&str>> {
        let index = register as usize;
        let Some(tag) = self.tags.get(index).copied() else {
            return Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            )));
        };

        match tag {
            RegTag::Object => Ok(Some(self.boxed[index].as_deref().ok_or_else(|| {
                WinedroidError::new(format!("DEX register v{register} lost its object class"))
            })?)),
            RegTag::Reference => {
                let reference = self.boxed[index].as_deref().ok_or_else(|| {
                    WinedroidError::new(format!(
                        "DEX register v{register} lost its reference payload"
                    ))
                })?;
                Ok(reference_descriptor(reference))
            }
            RegTag::String => Ok(Some("Ljava/lang/String;")),
            RegTag::Uninitialized => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            RegTag::Null | RegTag::Int | RegTag::Long | RegTag::Float => Ok(None),
        }
    }

    #[inline]
    pub(crate) fn read_int(&self, register: u16) -> Result<i32> {
        match self.tags.get(register as usize).copied() {
            Some(RegTag::Int) => Ok(self.words[register as usize] as u32 as i32),
            Some(RegTag::Uninitialized) => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            Some(_) => Err(WinedroidError::new(format!(
                "DEX register v{register} does not contain an int"
            ))),
            None => Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            ))),
        }
    }

    #[inline]
    pub(crate) fn read_long(&self, register: u16) -> Result<i64> {
        match self.tags.get(register as usize).copied() {
            Some(RegTag::Long) => Ok(self.words[register as usize] as i64),
            Some(RegTag::Uninitialized) => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            Some(_) => Err(WinedroidError::new(format!(
                "DEX register v{register} does not contain a long"
            ))),
            None => Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            ))),
        }
    }

    pub(crate) fn read_float(&self, register: u16) -> Result<f32> {
        match self.tags.get(register as usize).copied() {
            Some(RegTag::Float) => Ok(f32::from_bits(self.words[register as usize] as u32)),
            Some(RegTag::Int) => Ok(f32::from_bits(self.words[register as usize] as u32)),
            Some(RegTag::Uninitialized) => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            Some(_) => Err(WinedroidError::new(format!(
                "DEX register v{register} does not contain a float"
            ))),
            None => Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            ))),
        }
    }

    fn read_double(&self, register: u16) -> Result<f64> {
        match self.tags.get(register as usize).copied() {
            Some(RegTag::Long) => Ok(f64::from_bits(self.words[register as usize])),
            Some(RegTag::Uninitialized) => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            Some(_) => Err(WinedroidError::new(format!(
                "DEX register v{register} does not contain a double"
            ))),
            None => Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            ))),
        }
    }

    #[inline]
    fn is_zero(&self, register: u16) -> Result<bool> {
        match self.tags.get(register as usize).copied() {
            Some(RegTag::Null) => Ok(true),
            Some(RegTag::Int) => Ok(self.words[register as usize] as u32 as i32 == 0),
            Some(RegTag::String) | Some(RegTag::Object) | Some(RegTag::Reference) => Ok(false),
            Some(RegTag::Uninitialized) => Err(WinedroidError::new(format!(
                "DEX register v{register} is uninitialized"
            ))),
            Some(_) => Err(WinedroidError::new(format!(
                "DEX register v{register} does not contain an int or reference"
            ))),
            None => Err(WinedroidError::new(format!(
                "DEX register v{register} is outside the frame"
            ))),
        }
    }
}

fn reference_descriptor(reference: &str) -> Option<&str> {
    if !reference.starts_with('L') {
        return None;
    }

    let end = reference.find(';')?;
    reference.get(..=end)
}

#[cfg(test)]
fn interpret_code_with_handler(
    code: &DexCodeItem,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    interpret_code_with_initial_arguments(code, None, invocation_handler)
}

#[cfg(test)]
fn interpret_code_with_initial_arguments(
    code: &DexCodeItem,
    initial_arguments: Option<&[DexValue]>,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    interpret_code_with_initial_arguments_in_context(code, initial_arguments, invocation_handler, 0)
}

pub fn interpret_code_with_initial_arguments_in_context(
    code: &DexCodeItem,
    initial_arguments: Option<&[DexValue]>,
    invocation_handler: &mut dyn DexInvocationHandler,
    dex_file_index: usize,
) -> Result<DexExecutionResult> {
    let mut frame = DexFrame::new(code.registers_size);
    initialize_parameter_registers(&mut frame, code, initial_arguments)?;

    continue_code(
        code,
        frame,
        None,
        None,
        0,
        None,
        dex_file_index,
        invocation_handler,
    )
}

pub(crate) fn interpret_code_with_frame_in_context(
    code: &DexCodeItem,
    frame: DexFrame,
    invocation_handler: &mut dyn DexInvocationHandler,
    dex_file_index: usize,
) -> Result<DexExecutionResult> {
    continue_code(
        code,
        frame,
        None,
        None,
        0,
        None,
        dex_file_index,
        invocation_handler,
    )
}

#[cfg(all(feature = "jit", not(target_family = "wasm")))]
pub fn interpret_code_from_pc(
    code: &DexCodeItem,
    dex_file_index: usize,
    registers: Vec<DexValue>,
    pc: usize,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    continue_code(
        code,
        DexFrame::from_values(registers),
        None,
        None,
        pc,
        None,
        dex_file_index,
        invocation_handler,
    )
}

#[cfg(all(feature = "jit", not(target_family = "wasm")))]
pub fn complete_invocation_from_pc(
    code: &DexCodeItem,
    dex_file_index: usize,
    registers: Vec<DexValue>,
    pc: usize,
    invocation_result: DexInvocationResult,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    continue_code(
        code,
        DexFrame::from_values(registers),
        None,
        None,
        pc,
        Some(DexFrameResume::Invocation(invocation_result)),
        dex_file_index,
        invocation_handler,
    )
}

#[cfg(all(feature = "jit", not(target_family = "wasm")))]
pub fn park_caller_from_registers(
    execution: &mut DexParkedExecution,
    dex_file_index: usize,
    current_class_descriptor: Option<&str>,
    code: &DexCodeItem,
    registers: Vec<DexValue>,
    pc: usize,
) {
    execution.push_caller(DexParkedFrame {
        dex_file_index,
        current_class_descriptor: current_class_descriptor.map(str::to_owned),
        code: code.clone(),
        frame: DexFrame::from_values(registers),
        pending_result: None,
        pending_exception: None,
        pc,
        resume_kind: DexParkedFrameResumeKind::Invocation,
    });
}

pub fn resume_parked_frame(
    frame: DexParkedFrame,
    invocation_result: DexInvocationResult,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    let code = frame.code.clone();
    let resume = match frame.resume_kind {
        DexParkedFrameResumeKind::Invocation => DexFrameResume::Invocation(invocation_result),
        DexParkedFrameResumeKind::RetryInstruction => {
            DexFrameResume::RetryInstruction(invocation_result)
        }
    };
    continue_code(
        &code,
        frame.frame,
        frame.pending_result,
        frame.pending_exception,
        frame.pc,
        Some(resume),
        frame.dex_file_index,
        invocation_handler,
    )
}

enum DexFrameResume {
    Invocation(DexInvocationResult),
    RetryInstruction(DexInvocationResult),
}

fn continue_code(
    code: &DexCodeItem,
    mut frame: DexFrame,
    mut pending_result: Option<DexPendingResult>,
    mut pending_exception: Option<DexValue>,
    mut pc: usize,
    resume: Option<DexFrameResume>,
    dex_file_index: usize,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<DexExecutionResult> {
    let record_opcodes = opcode_histogram_enabled() || opcode_step_histogram_enabled();

    if let Some(resume) = resume {
        let finished = match resume {
            DexFrameResume::Invocation(result) => complete_invocation_result(
                code,
                dex_file_index,
                &frame,
                &mut pending_result,
                &mut pending_exception,
                &mut pc,
                result,
                invocation_handler,
            )?,
            DexFrameResume::RetryInstruction(DexInvocationResult::Void) => None,
            DexFrameResume::RetryInstruction(DexInvocationResult::Throw(exception)) => {
                dispatch_exception(
                    code,
                    pc,
                    exception,
                    &mut pending_result,
                    &mut pending_exception,
                    &mut pc,
                    invocation_handler,
                )?
            }
            DexFrameResume::RetryInstruction(result) => {
                return Err(WinedroidError::new(format!(
                    "parked DEX instruction resumed with invalid result {result:?}"
                )));
            }
        };
        if let Some(finished) = finished {
            return Ok(finished);
        }
    }

    loop {
        let Some(instruction) = code.instructions.get(pc).copied() else {
            return Err(WinedroidError::new(
                "DEX method reached the end of instructions without returning",
            ));
        };

        let opcode = (instruction & 0x00ff) as u8;
        if record_opcodes {
            record_opcode_histogram(opcode);
        }
        match opcode {
            0x00 => {
                pc += 1;
            }
            0x01 | 0x07 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                frame.copy_register(destination, source)?;
                pc += 1;
            }
            0x02 | 0x08 => {
                let destination = u16::from(high_byte(instruction));
                let source = read_next_code_unit(code, pc)?;
                frame.copy_register(destination, source)?;
                pc += 2;
            }
            0x03 | 0x09 => {
                let destination = read_next_code_unit(code, pc)?;
                let source = read_code_unit(code, pc + 2)?;
                frame.copy_register(destination, source)?;
                pc += 3;
            }
            0x04 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_long(source)?;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x05 => {
                let destination = u16::from(high_byte(instruction));
                let source = read_next_code_unit(code, pc)?;
                let value = frame.read_long(source)?;
                frame.set_long_pair(destination, value)?;
                pc += 2;
            }
            0x06 => {
                let destination = read_next_code_unit(code, pc)?;
                let source = read_code_unit(code, pc + 2)?;
                let value = frame.read_long(source)?;
                frame.set_long_pair(destination, value)?;
                pc += 3;
            }
            0x0e => {
                return Ok(DexExecutionResult::Void);
            }
            0x0a | 0x0c => {
                let register = u16::from(high_byte(instruction));
                let Some(result) = pending_result.take() else {
                    return Err(WinedroidError::new(format!(
                        "{} at instruction {pc} has no pending invocation result",
                        opcode_name(opcode)
                    )));
                };
                result.write_to_frame(&mut frame, register)?;
                pc += 1;
            }
            0x0b => {
                let register = u16::from(high_byte(instruction));
                let Some(result) = pending_result.take() else {
                    return Err(WinedroidError::new(format!(
                        "{} at instruction {pc} has no pending invocation result",
                        opcode_name(opcode)
                    )));
                };
                result.write_wide_to_frame(&mut frame, register)?;
                pc += 1;
            }
            0x0d => {
                let register = u16::from(high_byte(instruction));
                let Some(exception) = pending_exception.take() else {
                    return Err(WinedroidError::new(format!(
                        "move-exception at instruction {pc} has no pending exception"
                    )));
                };
                frame.set_register(register, exception)?;
                pc += 1;
            }
            0x0f => {
                let register = u16::from(high_byte(instruction));
                return match frame.read_value(register)? {
                    DexValue::Int(value) => Ok(DexExecutionResult::Int(value)),
                    DexValue::Float(value) => Ok(DexExecutionResult::Value(DexValue::Float(value))),
                    _ => Err(WinedroidError::new(format!(
                        "DEX register v{register} does not contain a single-width primitive"
                    ))),
                };
            }
            0x10 => {
                let register = u16::from(high_byte(instruction));
                return Ok(DexExecutionResult::Value(DexValue::Long(
                    frame.read_long(register)?,
                )));
            }
            0x11 => {
                let register = u16::from(high_byte(instruction));
                return Ok(DexExecutionResult::Value(frame.read_value(register)?));
            }
            0x12 => {
                let register = u16::from(nibble_a(instruction));
                let value = sign_extend_4(nibble_b(instruction));
                frame.set_int(register, value)?;
                pc += 1;
            }
            0x13 => {
                let register = u16::from(high_byte(instruction));
                let value = read_next_code_unit(code, pc)? as i16;
                frame.set_int(register, i32::from(value))?;
                pc += 2;
            }
            0x14 => {
                let register = u16::from(high_byte(instruction));
                let low = u32::from(read_next_code_unit(code, pc)?);
                let high = u32::from(read_code_unit(code, pc + 2)?);
                let value = low | (high << 16);
                frame.set_int(register, value as i32)?;
                pc += 3;
            }
            0x15 => {
                let register = u16::from(high_byte(instruction));
                let value = i32::from(read_next_code_unit(code, pc)? as i16) << 16;
                frame.set_int(register, value)?;
                pc += 2;
            }
            0x16 => {
                let register = u16::from(high_byte(instruction));
                let value = i64::from(read_next_code_unit(code, pc)? as i16);
                frame.set_long_pair(register, value)?;
                pc += 2;
            }
            0x17 => {
                let register = u16::from(high_byte(instruction));
                let low = u32::from(read_next_code_unit(code, pc)?);
                let high = u32::from(read_code_unit(code, pc + 2)?);
                let value = (low | (high << 16)) as i32;
                frame.set_long_pair(register, i64::from(value))?;
                pc += 3;
            }
            0x18 => {
                let register = u16::from(high_byte(instruction));
                let low_0 = u64::from(read_next_code_unit(code, pc)?);
                let low_1 = u64::from(read_code_unit(code, pc + 2)?);
                let high_0 = u64::from(read_code_unit(code, pc + 3)?);
                let high_1 = u64::from(read_code_unit(code, pc + 4)?);
                let value = low_0 | (low_1 << 16) | (high_0 << 32) | (high_1 << 48);
                frame.set_long_pair(register, value as i64)?;
                pc += 5;
            }
            0x19 => {
                let register = u16::from(high_byte(instruction));
                let value = i64::from(read_next_code_unit(code, pc)? as i16) << 48;
                frame.set_long_pair(register, value)?;
                pc += 2;
            }
            0x1a => {
                let access = read_string_access(code, pc, instruction)?;
                let destination_register = access.destination_register;
                let value = invocation_handler.load_string(access)?;
                frame.set_register(destination_register, value)?;
                pc += 2;
            }
            0x1b => {
                let access = read_jumbo_string_access(code, pc, instruction)?;
                let destination_register = access.destination_register;
                let value = invocation_handler.load_string(access)?;
                frame.set_register(destination_register, value)?;
                pc += 3;
            }
            0x1c => {
                let access = read_class_access(code, pc, instruction)?;
                let destination_register = access.destination_register;
                let value = invocation_handler.load_class(access)?;
                frame.set_register(destination_register, value)?;
                pc += 2;
            }
            0x1d | 0x1e => {
                let register = u16::from(high_byte(instruction));
                frame.read_value(register)?;
                pc += 1;
            }
            0x1f => {
                let check = read_type_check(code, &frame, pc, instruction)?;
                // cite: art interpreter_switch_impl-inl.h#CHECK_CAST
                match invocation_handler.check_cast(check)? {
                    DexCheckCastResult::Success => pc += 2,
                    DexCheckCastResult::Throw(exception) => {
                        let finished = dispatch_generated_exception(
                            code,
                            pc,
                            exception,
                            &mut pending_result,
                            &mut pending_exception,
                            &mut pc,
                            invocation_handler,
                        )?;
                        if let Some(finished) = finished {
                            return Ok(finished);
                        }
                    }
                }
            }
            0x20 => {
                let check = read_instance_of(code, &frame, pc, instruction)?;
                let destination_register = check.destination_register;
                let value = invocation_handler.instance_of(check)?;
                frame.set_int(destination_register, value)?;
                pc += 2;
            }
            0x21 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let destination_register = u16::from(nibble_a(instruction));
                let array_register = u16::from(nibble_b(instruction));
                let array = frame.read_value(array_register)?;
                let value = invocation_handler.array_length(DexArrayLength {
                    destination_register,
                    array_register,
                    array,
                })?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                frame.set_int(destination_register, value)?;
                pc += 1;
            }
            0x22 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let instance = read_new_instance(code, pc, instruction)?;
                let destination_register = instance.destination_register;
                // cite: art interpreter_switch_impl-inl.h#NEW_INSTANCE
                let allocated = invocation_handler.allocate_object(instance)?;
                crate::method_metrics::record_object_alloc_elapsed(method_metric_start);
                match allocated {
                    DexNewInstanceResult::Allocated(value) => {
                        frame.set_register(destination_register, value)?;
                        pc += 2;
                    }
                    DexNewInstanceResult::Throw(exception) => {
                        let finished = dispatch_generated_exception(
                            code,
                            pc,
                            exception,
                            &mut pending_result,
                            &mut pending_exception,
                            &mut pc,
                            invocation_handler,
                        )?;
                        if let Some(finished) = finished {
                            return Ok(finished);
                        }
                    }
                }
            }
            0x23 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let array = read_new_array(code, &frame, pc, instruction)?;
                let destination_register = array.destination_register;
                let value = invocation_handler.allocate_array(array)?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                frame.set_register(destination_register, value)?;
                pc += 2;
            }
            0x24 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let array = read_filled_new_array(code, &frame, pc, instruction)?;
                let value = invocation_handler.allocate_filled_array(array)?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                pending_result = Some(DexPendingResult::from_value(value));
                pc += 3;
            }
            0x25 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let array = read_filled_new_array_range(code, &frame, pc, instruction)?;
                let value = invocation_handler.allocate_filled_array(array)?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                pending_result = Some(DexPendingResult::from_value(value));
                pc += 3;
            }
            0x26 => {
                let method_metric_start = crate::method_metrics::start_timing();
                fill_array_data(code, &frame, pc, instruction, invocation_handler)?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                pc += 3;
            }
            0x27 => {
                let register = u16::from(high_byte(instruction));
                let value = frame.read_value(register)?;

                // cite: art interpreter_switch_impl-inl.h#THROW, HandlePendingException
                // cite: art common_throws.cc#ThrowNullPointerException
                let finished = match value {
                    DexValue::Null | DexValue::Int(0) => dispatch_generated_exception(
                        code,
                        pc,
                        DexGeneratedException {
                            class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
                            detail_message: None,
                        },
                        &mut pending_result,
                        &mut pending_exception,
                        &mut pc,
                        invocation_handler,
                    )?,
                    DexValue::Object(_) | DexValue::Reference(_) => dispatch_exception(
                        code,
                        pc,
                        value,
                        &mut pending_result,
                        &mut pending_exception,
                        &mut pc,
                        invocation_handler,
                    )?,
                    value => {
                        return Err(WinedroidError::new(format!(
                            "DEX throw expected an object reference in v{register} at instruction {pc}; found {value}"
                        )));
                    }
                };
                if let Some(finished) = finished {
                    return Ok(finished);
                }
            }
            0x28 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let branch_offset = high_byte(instruction) as i8;
                pc = branch_target(code, pc, isize::from(branch_offset))?;
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x29 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let branch_offset = read_next_code_unit(code, pc)? as i16;
                pc = branch_target(code, pc, isize::from(branch_offset))?;
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x2a => {
                let method_metric_start = crate::method_metrics::start_timing();
                let low = u32::from(read_next_code_unit(code, pc)?);
                let high = u32::from(read_code_unit(code, pc + 2)?);
                let branch_offset = (low | (high << 16)) as i32;
                pc = branch_target(code, pc, branch_offset as isize)?;
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x2b => {
                let method_metric_start = crate::method_metrics::start_timing();
                let target = read_packed_switch_target(code, &frame, pc, instruction)?;

                if let Some(branch_offset) = target {
                    pc = branch_target(code, pc, branch_offset)?;
                } else {
                    pc += 3;
                }
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x2c => {
                let method_metric_start = crate::method_metrics::start_timing();
                let target = read_sparse_switch_target(code, &frame, pc, instruction)?;

                if let Some(branch_offset) = target {
                    pc = branch_target(code, pc, branch_offset)?;
                } else {
                    pc += 3;
                }
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x2d..=0x2e => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_float(left_register)?;
                let right = frame.read_float(right_register)?;
                let value = float_compare_result(left, right, opcode)?;
                frame.set_int(destination, value)?;
                pc += 2;
            }
            0x2f..=0x30 => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_double(left_register)?;
                let right = frame.read_double(right_register)?;
                let value = double_compare_result(left, right, opcode)?;
                frame.set_int(destination, value)?;
                pc += 2;
            }
            0x31 => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_long(left_register)?;
                let right = frame.read_long(right_register)?;
                let value = match left.cmp(&right) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                frame.set_int(destination, value)?;
                pc += 2;
            }
            0x32..=0x37 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let left_register = u16::from(nibble_a(instruction));
                let right_register = u16::from(nibble_b(instruction));
                let branch_offset = read_branch_offset(code, pc)?;
                let should_branch =
                    compare_branch_matches(&frame, left_register, right_register, opcode)?;

                if should_branch {
                    pc = branch_target(code, pc, isize::from(branch_offset))?;
                } else {
                    pc += 2;
                }
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x38..=0x3d => {
                let method_metric_start = crate::method_metrics::start_timing();
                let register = u16::from(high_byte(instruction));
                let branch_offset = read_branch_offset(code, pc)?;
                let should_branch = zero_branch_matches(&frame, register, opcode)?;

                if should_branch {
                    pc = branch_target(code, pc, isize::from(branch_offset))?;
                } else {
                    pc += 2;
                }
                crate::method_metrics::record_branch_elapsed(method_metric_start);
            }
            0x44..=0x4a => {
                let method_metric_start = crate::method_metrics::start_timing();
                let destination_register = u16::from(high_byte(instruction));
                let array_and_index = read_next_code_unit(code, pc)?;
                let array_register = array_and_index & 0x00ff;
                let index_register = array_and_index >> 8;
                let array = frame.read_value(array_register)?;
                let index = frame.read_int(index_register)?;
                let value = invocation_handler.read_array(DexArrayRead {
                    destination_register,
                    array_register,
                    index_register,
                    array,
                    index,
                })?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                frame.set_register(destination_register, value)?;
                if opcode == 0x45 {
                    let value = frame.read_long(destination_register)?;
                    frame.set_long(destination_register + 1, value)?;
                }
                pc += 2;
            }
            0x4b..=0x51 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let value_register = u16::from(high_byte(instruction));
                let array_and_index = read_next_code_unit(code, pc)?;
                let array_register = array_and_index & 0x00ff;
                let index_register = array_and_index >> 8;
                let value = frame.read_value(value_register)?;
                let array = frame.read_value(array_register)?;
                let index = frame.read_int(index_register)?;

                invocation_handler.write_array(DexArrayWrite {
                    value_register,
                    array_register,
                    index_register,
                    value,
                    array,
                    index,
                })?;
                crate::method_metrics::record_array_op_elapsed(method_metric_start);
                pc += 2;
            }
            0x52..=0x58 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let destination_register = u16::from(nibble_a(instruction));
                let object_register = u16::from(nibble_b(instruction));
                let field_index = u32::from(read_next_code_unit(code, pc)?);
                let object = frame.read_value(object_register)?;
                let value = invocation_handler.read_instance_field(DexFieldAccess {
                    field_index,
                    destination_register,
                    object_register,
                    object,
                })?;
                crate::method_metrics::record_field_op_elapsed(method_metric_start);
                if opcode == 0x53 {
                    let DexValue::Long(value) = value else {
                        return Err(WinedroidError::new(format!(
                            "iget-wide expected a long field value at instruction {pc}"
                        )));
                    };
                    frame.set_long_pair(destination_register, value)?;
                } else {
                    frame.set_register(destination_register, value)?;
                }
                pc += 2;
            }
            0x59..=0x5f => {
                let method_metric_start = crate::method_metrics::start_timing();
                let value_register = u16::from(nibble_a(instruction));
                let object_register = u16::from(nibble_b(instruction));
                let field_index = u32::from(read_next_code_unit(code, pc)?);
                let value = frame.read_value(value_register)?;
                let object = frame.read_value(object_register)?;
                invocation_handler.write_instance_field(DexFieldWrite {
                    field_index,
                    value_register,
                    object_register,
                    value,
                    object,
                })?;
                crate::method_metrics::record_field_op_elapsed(method_metric_start);
                pc += 2;
            }
            0x60..=0x66 => {
                let method_metric_start = crate::method_metrics::start_timing();
                let access = read_static_field_access(code, pc, instruction)?;
                let destination_register = access.destination_register;
                let result = invocation_handler.read_static_field(access)?;
                crate::method_metrics::record_field_op_elapsed(method_metric_start);
                let value = match result {
                    DexStaticFieldReadResult::Value(value) => value,
                    DexStaticFieldReadResult::Throw(exception) => {
                        if let Some(finished) = dispatch_exception(
                            code,
                            pc,
                            exception,
                            &mut pending_result,
                            &mut pending_exception,
                            &mut pc,
                            invocation_handler,
                        )? {
                            return Ok(finished);
                        }
                        continue;
                    }
                    DexStaticFieldReadResult::Parked(mut execution) => {
                        let current_class_descriptor = invocation_handler
                            .current_class_descriptor()
                            .map(str::to_owned);
                        execution.push_caller(DexParkedFrame::retry_instruction(
                            dex_file_index,
                            current_class_descriptor,
                            code,
                            &frame,
                            &pending_result,
                            &pending_exception,
                            pc,
                        ));
                        return Ok(DexExecutionResult::Parked(execution));
                    }
                };
                if opcode == 0x61 {
                    let DexValue::Long(value) = value else {
                        return Err(WinedroidError::new(format!(
                            "sget-wide expected a long field value at instruction {pc}"
                        )));
                    };
                    frame.set_long_pair(destination_register, value)?;
                } else {
                    frame.set_register(destination_register, value)?;
                }
                pc += 2;
            }
            0x67..=0x6d => {
                let method_metric_start = crate::method_metrics::start_timing();
                let write = read_static_field_write(code, &frame, pc, instruction)?;
                let result = invocation_handler.write_static_field(write)?;
                crate::method_metrics::record_field_op_elapsed(method_metric_start);
                match result {
                    DexStaticFieldWriteResult::Complete => pc += 2,
                    DexStaticFieldWriteResult::Throw(exception) => {
                        if let Some(finished) = dispatch_exception(
                            code,
                            pc,
                            exception,
                            &mut pending_result,
                            &mut pending_exception,
                            &mut pc,
                            invocation_handler,
                        )? {
                            return Ok(finished);
                        }
                    }
                    DexStaticFieldWriteResult::Parked(mut execution) => {
                        let current_class_descriptor = invocation_handler
                            .current_class_descriptor()
                            .map(str::to_owned);
                        execution.push_caller(DexParkedFrame::retry_instruction(
                            dex_file_index,
                            current_class_descriptor,
                            code,
                            &frame,
                            &pending_result,
                            &pending_exception,
                            pc,
                        ));
                        return Ok(DexExecutionResult::Parked(execution));
                    }
                }
            }
            0x6e..=0x72 => {
                let invocation =
                    read_five_register_invocation_frame(code, &frame, pc, instruction, opcode)?;
                let method_metric_start = crate::method_metrics::start_timing();
                let invocation_result = invocation_handler.invoke_from_frame(invocation);
                crate::method_metrics::record_call_elapsed(method_metric_start);
                let invocation_result = invocation_result?;
                if let Some(finished) = complete_invocation_result(
                    code,
                    dex_file_index,
                    &frame,
                    &mut pending_result,
                    &mut pending_exception,
                    &mut pc,
                    invocation_result,
                    invocation_handler,
                )? {
                    return Ok(finished);
                }
            }
            0x74..=0x78 => {
                let invocation =
                    read_range_invocation_frame(code, &frame, pc, instruction, opcode)?;
                let method_metric_start = crate::method_metrics::start_timing();
                let invocation_result = invocation_handler.invoke_from_frame(invocation);
                crate::method_metrics::record_call_elapsed(method_metric_start);
                let invocation_result = invocation_result?;
                if let Some(finished) = complete_invocation_result(
                    code,
                    dex_file_index,
                    &frame,
                    &mut pending_result,
                    &mut pending_exception,
                    &mut pc,
                    invocation_result,
                    invocation_handler,
                )? {
                    return Ok(finished);
                }
            }
            0x7d | 0x7e => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let source_value = frame.read_long(source)?;
                let value = long_unary_result(source_value, opcode)?;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x7f => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = -frame.read_float(source)?;
                frame.set_float(destination, value)?;
                pc += 1;
            }
            0x80 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = (-frame.read_double(source)?).to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x81 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = i64::from(frame.read_int(source)?);
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x82 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_int(source)? as f32;
                frame.set_float(destination, value)?;
                pc += 1;
            }
            0x83 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = f64::from(frame.read_int(source)?).to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x84 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_long(source)? as i32;
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x85 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_long(source)? as f32;
                frame.set_float(destination, value)?;
                pc += 1;
            }
            0x86 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = (frame.read_long(source)? as f64).to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x8d => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = i32::from(frame.read_int(source)? as i8);
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x8e => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = i32::from(frame.read_int(source)? as u16);
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x8f => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = i32::from(frame.read_int(source)? as i16);
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x7b | 0x7c => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let source_value = frame.read_int(source)?;
                let value = int_unary_result(source_value, opcode)?;
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x87 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_float(source)? as i32;
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x88 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_float(source)? as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x89 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = f64::from(frame.read_float(source)?).to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x8a => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_double(source)? as i32;
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0x8b => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_double(source)? as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0x8c => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let value = frame.read_double(source)? as f32;
                frame.set_float(destination, value)?;
                pc += 1;
            }
            0x90..=0x9a => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_int(left_register)?;
                let right = frame.read_int(right_register)?;
                let value = int_binary_result(left, right, opcode)?;
                frame.set_int(destination, value)?;
                pc += 2;
            }
            0x9b..=0xa2 => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_long(left_register)?;
                let right = frame.read_long(right_register)?;
                let value = long_binary_result(left, right, opcode)?;
                frame.set_long_pair(destination, value)?;
                pc += 2;
            }
            0xa3..=0xa5 => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_long(left_register)?;
                let right = frame.read_int(right_register)?;
                let value = long_shift_result(left, right, opcode)?;
                frame.set_long_pair(destination, value)?;
                pc += 2;
            }
            0xa6..=0xaa => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_float(left_register)?;
                let right = frame.read_float(right_register)?;
                let value = float_binary_result(left, right, opcode)?;
                frame.set_float(destination, value)?;
                pc += 2;
            }
            0xab..=0xaf => {
                let destination = u16::from(high_byte(instruction));
                let source_registers = read_next_code_unit(code, pc)?;
                let left_register = source_registers & 0x00ff;
                let right_register = source_registers >> 8;
                let left = frame.read_double(left_register)?;
                let right = frame.read_double(right_register)?;
                let value = double_binary_result(left, right, opcode)?.to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 2;
            }
            0xb0..=0xba => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let left = frame.read_int(destination)?;
                let right = frame.read_int(source)?;
                let value = int_2addr_result(left, right, opcode)?;
                frame.set_int(destination, value)?;
                pc += 1;
            }
            0xbb..=0xc2 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let left = frame.read_long(destination)?;
                let right = frame.read_long(source)?;
                let value = long_2addr_result(left, right, opcode)?;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0xc3..=0xc5 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let left = frame.read_long(destination)?;
                let right = frame.read_int(source)?;
                let value = long_shift_2addr_result(left, right, opcode)?;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0xc6..=0xca => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let left = frame.read_float(destination)?;
                let right = frame.read_float(source)?;
                let value = float_2addr_result(left, right, opcode)?;
                frame.set_float(destination, value)?;
                pc += 1;
            }
            0xcb..=0xcf => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let left = frame.read_double(destination)?;
                let right = frame.read_double(source)?;
                let value = double_2addr_result(left, right, opcode)?.to_bits() as i64;
                frame.set_long_pair(destination, value)?;
                pc += 1;
            }
            0xd0..=0xd7 => {
                let destination = u16::from(nibble_a(instruction));
                let source = u16::from(nibble_b(instruction));
                let literal = read_next_code_unit(code, pc)? as i16;
                let source_value = frame.read_int(source)?;
                let value = int_lit16_result(source_value, literal, opcode)?;
                frame.set_int(destination, value)?;
                pc += 2;
            }
            0xd8..=0xe2 => {
                let destination = u16::from(high_byte(instruction));
                let source_and_literal = read_next_code_unit(code, pc)?;
                let source = source_and_literal & 0x00ff;
                let literal = (source_and_literal >> 8) as u8 as i8;
                let source_value = frame.read_int(source)?;
                let value = int_lit8_result(source_value, literal, opcode)?;
                frame.set_int(destination, value)?;
                pc += 2;
            }
            _ => {
                return Err(WinedroidError::new(format!(
                    "unsupported DEX opcode 0x{opcode:02x} ({}) at instruction {pc}",
                    opcode_name(opcode)
                )));
            }
        }
    }
}

fn complete_invocation_result(
    code: &DexCodeItem,
    dex_file_index: usize,
    frame: &DexFrame,
    pending_result: &mut Option<DexPendingResult>,
    pending_exception: &mut Option<DexValue>,
    pc: &mut usize,
    invocation_result: DexInvocationResult,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<Option<DexExecutionResult>> {
    match invocation_result {
        DexInvocationResult::Void => {
            *pending_result = None;
            *pc += 3;
            Ok(None)
        }
        DexInvocationResult::Value(value) => {
            *pending_result = Some(DexPendingResult::from_value(value));
            *pc += 3;
            Ok(None)
        }
        DexInvocationResult::RawValue(value) => {
            *pending_result = Some(value);
            *pc += 3;
            Ok(None)
        }
        DexInvocationResult::Throw(exception) => dispatch_exception(
            code,
            *pc,
            exception,
            pending_result,
            pending_exception,
            pc,
            invocation_handler,
        ),
        DexInvocationResult::Parked(mut execution) => {
            let current_class_descriptor = invocation_handler
                .current_class_descriptor()
                .map(str::to_owned);
            execution.push_caller(DexParkedFrame::new(
                dex_file_index,
                current_class_descriptor,
                code,
                frame,
                pending_result,
                pending_exception,
                *pc,
            ));
            Ok(Some(DexExecutionResult::Parked(execution)))
        }
    }
}

fn dispatch_generated_exception(
    code: &DexCodeItem,
    throwing_pc: usize,
    generated_exception: DexGeneratedException,
    pending_result: &mut Option<DexPendingResult>,
    pending_exception: &mut Option<DexValue>,
    pc: &mut usize,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<Option<DexExecutionResult>> {
    let class_descriptor = generated_exception.class_descriptor.clone();
    let exception = invocation_handler.allocate_generated_exception(generated_exception)?;
    if !matches!(exception, DexValue::Object(_)) {
        return Err(WinedroidError::new(format!(
            "DEX generated exception allocation for {class_descriptor} returned {exception}"
        )));
    }

    dispatch_exception(
        code,
        throwing_pc,
        exception,
        pending_result,
        pending_exception,
        pc,
        invocation_handler,
    )
}

fn dispatch_exception(
    code: &DexCodeItem,
    throwing_pc: usize,
    exception: DexValue,
    pending_result: &mut Option<DexPendingResult>,
    pending_exception: &mut Option<DexValue>,
    pc: &mut usize,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<Option<DexExecutionResult>> {
    *pending_result = None;

    // cite: art interpreter_switch_impl-inl.h#HandlePendingException, PossiblyHandlePendingExceptionOnInvoke
    // cite: art interpreter_common.cc#MoveToExceptionHandler
    // cite: art art_method.cc#FindCatchBlock
    let Some(handler_pc) = exception_handler_pc(code, throwing_pc, &exception, invocation_handler)?
    else {
        *pending_exception = None;
        return Ok(Some(DexExecutionResult::Throw(exception)));
    };

    let handler_opcode = (code.instructions[handler_pc] & 0x00ff) as u8;
    *pending_exception = (handler_opcode == 0x0d).then_some(exception);
    *pc = handler_pc;
    Ok(None)
}

fn exception_handler_pc(
    code: &DexCodeItem,
    pc: usize,
    exception: &DexValue,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<Option<usize>> {
    let pc = u32::try_from(pc).map_err(|_| WinedroidError::new("DEX pc overflow"))?;

    // cite: art art_method.cc#FindCatchBlock
    for try_block in &code.try_blocks {
        let start = try_block.start_address;
        let end = start
            .checked_add(u32::from(try_block.instruction_count))
            .ok_or_else(|| WinedroidError::new("DEX try block range overflow"))?;

        if pc < start || pc >= end {
            continue;
        }

        for handler in &try_block.handlers {
            let matches = match &handler.type_descriptor {
                Some(type_descriptor) => {
                    invocation_handler.exception_matches(exception, type_descriptor)?
                }
                None => true,
            };

            if matches {
                let handler_pc = usize::try_from(handler.address)
                    .map_err(|_| WinedroidError::new("DEX exception handler pc overflow"))?;
                if handler_pc >= code.instructions.len() {
                    return Err(WinedroidError::new(format!(
                        "DEX exception handler pc {handler_pc} is outside the instruction stream"
                    )));
                }

                return Ok(Some(handler_pc));
            }
        }
    }

    Ok(None)
}

fn zero_branch_matches(frame: &DexFrame, register: u16, opcode: u8) -> Result<bool> {
    match opcode {
        0x38 => frame.is_zero(register),
        0x39 => frame.is_zero(register).map(|value| !value),
        0x3a => frame.read_int(register).map(|value| value < 0),
        0x3b => frame.read_int(register).map(|value| value >= 0),
        0x3c => frame.read_int(register).map(|value| value > 0),
        0x3d => frame.read_int(register).map(|value| value <= 0),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a zero-branch opcode"
        ))),
    }
}

fn compare_branch_matches(
    frame: &DexFrame,
    left_register: u16,
    right_register: u16,
    opcode: u8,
) -> Result<bool> {
    match opcode {
        0x32 => Ok(frame.read_value(left_register)? == frame.read_value(right_register)?),
        0x33 => Ok(frame.read_value(left_register)? != frame.read_value(right_register)?),
        0x34 => Ok(frame.read_int(left_register)? < frame.read_int(right_register)?),
        0x35 => Ok(frame.read_int(left_register)? >= frame.read_int(right_register)?),
        0x36 => Ok(frame.read_int(left_register)? > frame.read_int(right_register)?),
        0x37 => Ok(frame.read_int(left_register)? <= frame.read_int(right_register)?),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a compare-branch opcode"
        ))),
    }
}

fn float_compare_result(left: f32, right: f32, opcode: u8) -> Result<i32> {
    if left.is_nan() || right.is_nan() {
        return match opcode {
            0x2d => Ok(-1),
            0x2e => Ok(1),
            _ => Err(WinedroidError::new(format!(
                "opcode 0x{opcode:02x} is not a float compare opcode"
            ))),
        };
    }

    match left.partial_cmp(&right) {
        Some(std::cmp::Ordering::Less) => Ok(-1),
        Some(std::cmp::Ordering::Equal) => Ok(0),
        Some(std::cmp::Ordering::Greater) => Ok(1),
        None => Err(WinedroidError::new(
            "float compare failed on non-NaN values",
        )),
    }
}

fn double_compare_result(left: f64, right: f64, opcode: u8) -> Result<i32> {
    if left.is_nan() || right.is_nan() {
        return match opcode {
            0x2f => Ok(-1),
            0x30 => Ok(1),
            _ => Err(WinedroidError::new(format!(
                "opcode 0x{opcode:02x} is not a double compare opcode"
            ))),
        };
    }

    match left.partial_cmp(&right) {
        Some(std::cmp::Ordering::Less) => Ok(-1),
        Some(std::cmp::Ordering::Equal) => Ok(0),
        Some(std::cmp::Ordering::Greater) => Ok(1),
        None => Err(WinedroidError::new(
            "double compare failed on non-NaN values",
        )),
    }
}

fn int_binary_result(left: i32, right: i32, opcode: u8) -> Result<i32> {
    match opcode {
        0x90 => Ok(left.wrapping_add(right)),
        0x91 => Ok(left.wrapping_sub(right)),
        0x92 => Ok(left.wrapping_mul(right)),
        0x93 => {
            if right == 0 {
                return Err(WinedroidError::new("DEX div-int by zero"));
            }

            Ok(left.wrapping_div(right))
        }
        0x94 => {
            if right == 0 {
                return Err(WinedroidError::new("DEX rem-int by zero"));
            }

            Ok(left.wrapping_rem(right))
        }
        0x95 => Ok(left & right),
        0x96 => Ok(left | right),
        0x97 => Ok(left ^ right),
        0x98 => Ok(left.wrapping_shl((right & 0x1f) as u32)),
        0x99 => Ok(left.wrapping_shr((right & 0x1f) as u32)),
        0x9a => Ok(((left as u32).wrapping_shr((right & 0x1f) as u32)) as i32),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an int binary opcode"
        ))),
    }
}

fn int_unary_result(source: i32, opcode: u8) -> Result<i32> {
    match opcode {
        0x7b => Ok(source.wrapping_neg()),
        0x7c => Ok(!source),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an int unary opcode"
        ))),
    }
}

fn long_unary_result(source: i64, opcode: u8) -> Result<i64> {
    match opcode {
        0x7d => Ok(source.wrapping_neg()),
        0x7e => Ok(!source),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a long unary opcode"
        ))),
    }
}

fn int_2addr_result(left: i32, right: i32, opcode: u8) -> Result<i32> {
    let Some(binary_opcode) = opcode.checked_sub(0x20) else {
        return Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an int/2addr opcode"
        )));
    };

    int_binary_result(left, right, binary_opcode)
}

fn long_binary_result(left: i64, right: i64, opcode: u8) -> Result<i64> {
    match opcode {
        0x9b => Ok(left.wrapping_add(right)),
        0x9c => Ok(left.wrapping_sub(right)),
        0x9d => Ok(left.wrapping_mul(right)),
        0x9e => {
            if right == 0 {
                return Err(WinedroidError::new("DEX div-long by zero"));
            }

            Ok(left.wrapping_div(right))
        }
        0x9f => {
            if right == 0 {
                return Err(WinedroidError::new("DEX rem-long by zero"));
            }

            Ok(left.wrapping_rem(right))
        }
        0xa0 => Ok(left & right),
        0xa1 => Ok(left | right),
        0xa2 => Ok(left ^ right),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a long binary opcode"
        ))),
    }
}

fn long_shift_result(left: i64, right: i32, opcode: u8) -> Result<i64> {
    let shift = (right & 0x3f) as u32;

    match opcode {
        0xa3 => Ok(left.wrapping_shl(shift)),
        0xa4 => Ok(left.wrapping_shr(shift)),
        0xa5 => Ok((left as u64).wrapping_shr(shift) as i64),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a long shift opcode"
        ))),
    }
}

fn long_2addr_result(left: i64, right: i64, opcode: u8) -> Result<i64> {
    let Some(binary_opcode) = opcode.checked_sub(0x20) else {
        return Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a long/2addr opcode"
        )));
    };

    long_binary_result(left, right, binary_opcode)
}

fn long_shift_2addr_result(left: i64, right: i32, opcode: u8) -> Result<i64> {
    let Some(shift_opcode) = opcode.checked_sub(0x20) else {
        return Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a long shift/2addr opcode"
        )));
    };

    long_shift_result(left, right, shift_opcode)
}

fn float_binary_result(left: f32, right: f32, opcode: u8) -> Result<f32> {
    match opcode {
        0xa6 => Ok(left + right),
        0xa7 => Ok(left - right),
        0xa8 => Ok(left * right),
        0xa9 => Ok(left / right),
        0xaa => Ok(left % right),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a float binary opcode"
        ))),
    }
}

fn double_binary_result(left: f64, right: f64, opcode: u8) -> Result<f64> {
    match opcode {
        0xab => Ok(left + right),
        0xac => Ok(left - right),
        0xad => Ok(left * right),
        0xae => Ok(left / right),
        0xaf => Ok(left % right),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a double binary opcode"
        ))),
    }
}

fn float_2addr_result(left: f32, right: f32, opcode: u8) -> Result<f32> {
    match opcode {
        0xc6 => Ok(left + right),
        0xc7 => Ok(left - right),
        0xc8 => Ok(left * right),
        0xc9 => Ok(left / right),
        0xca => Ok(left % right),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a float/2addr opcode"
        ))),
    }
}

fn double_2addr_result(left: f64, right: f64, opcode: u8) -> Result<f64> {
    match opcode {
        0xcb => Ok(left + right),
        0xcc => Ok(left - right),
        0xcd => Ok(left * right),
        0xce => Ok(left / right),
        0xcf => Ok(left % right),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not a double/2addr opcode"
        ))),
    }
}

struct OpcodeHistogram {
    counts: [u64; 256],
    total: u64,
}

impl OpcodeHistogram {
    fn new() -> Self {
        Self {
            counts: [0; 256],
            total: 0,
        }
    }
}

impl Drop for OpcodeHistogram {
    fn drop(&mut self) {
        if self.total == 0 {
            return;
        }
        let mut entries = self
            .counts
            .iter()
            .enumerate()
            .filter_map(|(opcode, count)| (*count > 0).then_some((opcode as u8, *count)))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        eprintln!(
            "=== TOP OPCODES BY COUNT (n={}, distinct={}) ===",
            self.total,
            entries.len()
        );
        for (opcode, count) in entries.into_iter().take(40) {
            eprintln!("{count:8}  0x{opcode:02x} {}", opcode_name(opcode));
        }
    }
}

struct OpcodeStepHistogram {
    counts: [std::sync::atomic::AtomicU64; 256],
    total: std::sync::atomic::AtomicU64,
}

impl OpcodeStepHistogram {
    fn new() -> Self {
        Self {
            counts: std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0)),
            total: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

pub(crate) struct OpcodeStepHistogramEntry {
    pub(crate) opcode: u8,
    pub(crate) name: &'static str,
    pub(crate) count: u64,
}

pub(crate) struct OpcodeStepHistogramSnapshot {
    pub(crate) total: u64,
    pub(crate) entries: Vec<OpcodeStepHistogramEntry>,
}

fn opcode_histogram_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("WD_OPCODE_HISTOGRAM").is_some())
}

fn opcode_step_histogram_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("WD_OPCODE_STEP_HISTOGRAM").is_some())
}

fn opcode_step_histogram() -> &'static OpcodeStepHistogram {
    static HISTOGRAM: std::sync::OnceLock<OpcodeStepHistogram> = std::sync::OnceLock::new();
    HISTOGRAM.get_or_init(OpcodeStepHistogram::new)
}

pub(crate) fn reset_opcode_step_histogram() {
    if !opcode_step_histogram_enabled() {
        return;
    }
    let histogram = opcode_step_histogram();
    histogram
        .total
        .store(0, std::sync::atomic::Ordering::Relaxed);
    for count in &histogram.counts {
        count.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

pub(crate) fn drain_opcode_step_histogram(limit: usize) -> Option<OpcodeStepHistogramSnapshot> {
    if !opcode_step_histogram_enabled() {
        return None;
    }
    let histogram = opcode_step_histogram();
    let total = histogram
        .total
        .swap(0, std::sync::atomic::Ordering::Relaxed);
    let mut entries = histogram
        .counts
        .iter()
        .enumerate()
        .filter_map(|(opcode, count)| {
            let count = count.swap(0, std::sync::atomic::Ordering::Relaxed);
            (count > 0).then_some(OpcodeStepHistogramEntry {
                opcode: opcode as u8,
                name: opcode_name(opcode as u8),
                count,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.opcode.cmp(&right.opcode))
    });
    entries.truncate(limit);
    Some(OpcodeStepHistogramSnapshot { total, entries })
}

fn record_opcode_histogram(opcode: u8) {
    thread_local! {
        static OPCODE_HISTOGRAM: std::cell::RefCell<OpcodeHistogram> =
            std::cell::RefCell::new(OpcodeHistogram::new());
    }
    if opcode_histogram_enabled() {
        OPCODE_HISTOGRAM.with(|histogram| {
            let mut histogram = histogram.borrow_mut();
            histogram.counts[opcode as usize] += 1;
            histogram.total += 1;
        });
    }
    if opcode_step_histogram_enabled() {
        let histogram = opcode_step_histogram();
        histogram.counts[opcode as usize].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        histogram
            .total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn int_lit8_result(source: i32, literal: i8, opcode: u8) -> Result<i32> {
    let literal = i32::from(literal);

    match opcode {
        0xd8 => Ok(source.wrapping_add(literal)),
        0xd9 => Ok(literal.wrapping_sub(source)),
        0xda => Ok(source.wrapping_mul(literal)),
        0xdb => {
            if literal == 0 {
                return Err(WinedroidError::new("DEX div-int/lit8 by zero"));
            }

            Ok(source.wrapping_div(literal))
        }
        0xdc => {
            if literal == 0 {
                return Err(WinedroidError::new("DEX rem-int/lit8 by zero"));
            }

            Ok(source.wrapping_rem(literal))
        }
        0xdd => Ok(source & literal),
        0xde => Ok(source | literal),
        0xdf => Ok(source ^ literal),
        0xe0 => Ok(source.wrapping_shl((literal & 0x1f) as u32)),
        0xe1 => Ok(source.wrapping_shr((literal & 0x1f) as u32)),
        0xe2 => Ok(((source as u32) >> ((literal & 0x1f) as u32)) as i32),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an int/lit8 opcode"
        ))),
    }
}

fn int_lit16_result(source: i32, literal: i16, opcode: u8) -> Result<i32> {
    let literal = i32::from(literal);

    match opcode {
        0xd0 => Ok(source.wrapping_add(literal)),
        0xd1 => Ok(literal.wrapping_sub(source)),
        0xd2 => Ok(source.wrapping_mul(literal)),
        0xd3 => {
            if literal == 0 {
                return Err(WinedroidError::new("DEX div-int/lit16 by zero"));
            }

            Ok(source.wrapping_div(literal))
        }
        0xd4 => {
            if literal == 0 {
                return Err(WinedroidError::new("DEX rem-int/lit16 by zero"));
            }

            Ok(source.wrapping_rem(literal))
        }
        0xd5 => Ok(source & literal),
        0xd6 => Ok(source | literal),
        0xd7 => Ok(source ^ literal),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an int/lit16 opcode"
        ))),
    }
}

fn read_branch_offset(code: &DexCodeItem, pc: usize) -> Result<i16> {
    Ok(read_next_code_unit(code, pc)? as i16)
}

fn branch_target(code: &DexCodeItem, pc: usize, offset: isize) -> Result<usize> {
    let target = pc as isize + offset;

    if target < 0 || target as usize >= code.instructions.len() {
        return Err(WinedroidError::new(format!(
            "DEX branch target {target} is outside the method"
        )));
    }

    Ok(target as usize)
}

fn read_packed_switch_target(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<Option<isize>> {
    let register = u16::from(high_byte(instruction));
    let key = frame.read_int(register)?;
    let payload_offset = read_i32(code, pc + 1)?;
    let payload_pc = branch_target(code, pc, payload_offset as isize)?;
    let ident = read_code_unit(code, payload_pc)?;

    if ident != 0x0100 {
        return Err(WinedroidError::new(format!(
            "DEX packed-switch payload at instruction {payload_pc} has invalid ident 0x{ident:04x}"
        )));
    }

    let size = usize::from(read_code_unit(code, payload_pc + 1)?);
    let first_key = read_i32(code, payload_pc + 2)?;
    let Some(index) = key.checked_sub(first_key) else {
        return Ok(None);
    };

    if index < 0 || index as usize >= size {
        return Ok(None);
    }

    let target_pc = payload_pc + 4 + index as usize * 2;
    Ok(Some(read_i32(code, target_pc)? as isize))
}

fn read_sparse_switch_target(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<Option<isize>> {
    let register = u16::from(high_byte(instruction));
    let key = frame.read_int(register)?;
    let payload_offset = read_i32(code, pc + 1)?;
    let payload_pc = branch_target(code, pc, payload_offset as isize)?;
    let ident = read_code_unit(code, payload_pc)?;

    if ident != 0x0200 {
        return Err(WinedroidError::new(format!(
            "DEX sparse-switch payload at instruction {payload_pc} has invalid ident 0x{ident:04x}"
        )));
    }

    let size = usize::from(read_code_unit(code, payload_pc + 1)?);
    let keys_pc = payload_pc + 2;
    let targets_pc = keys_pc + size * 2;

    for index in 0..size {
        let key_pc = keys_pc + index * 2;
        let switch_key = read_i32(code, key_pc)?;

        if switch_key == key {
            let target_pc = targets_pc + index * 2;
            return Ok(Some(read_i32(code, target_pc)? as isize));
        }
    }

    Ok(None)
}

fn fill_array_data(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
    invocation_handler: &mut dyn DexInvocationHandler,
) -> Result<()> {
    let array_register = u16::from(high_byte(instruction));
    let array = frame.read_value(array_register)?;
    let payload_offset = read_i32(code, pc + 1)?;
    let payload_pc = branch_target(code, pc, payload_offset as isize)?;
    let ident = read_code_unit(code, payload_pc)?;

    if ident != 0x0300 {
        return Err(WinedroidError::new(format!(
            "DEX fill-array-data payload at instruction {payload_pc} has invalid ident 0x{ident:04x}"
        )));
    }

    let element_width = usize::from(read_code_unit(code, payload_pc + 1)?);
    let size = read_u32(code, payload_pc + 2)? as usize;
    let data_pc = payload_pc + 4;
    let byte_count = element_width
        .checked_mul(size)
        .ok_or_else(|| WinedroidError::new("DEX fill-array-data payload is too large"))?;
    let required_code_units = byte_count.div_ceil(2);
    require_code_range(
        code,
        data_pc,
        required_code_units,
        "fill-array-data payload",
    )?;

    for index in 0..size {
        let value = read_fill_array_element(code, data_pc, index * element_width, element_width)?;
        invocation_handler.write_array(DexArrayWrite {
            value_register: array_register,
            array_register,
            index_register: array_register,
            value,
            array: array.clone(),
            index: i32::try_from(index).map_err(|_| {
                WinedroidError::new("DEX fill-array-data index is outside i32 range")
            })?,
        })?;
    }

    Ok(())
}

fn read_fill_array_element(
    code: &DexCodeItem,
    data_pc: usize,
    byte_offset: usize,
    element_width: usize,
) -> Result<DexValue> {
    match element_width {
        1 => Ok(DexValue::Int(i32::from(
            read_payload_u8(code, data_pc, byte_offset)? as i8,
        ))),
        2 => Ok(DexValue::Int(i32::from(
            read_payload_u16(code, data_pc, byte_offset)? as i16,
        ))),
        4 => Ok(DexValue::Int(
            read_payload_u32(code, data_pc, byte_offset)? as i32
        )),
        8 => Ok(DexValue::Long(
            read_payload_u64(code, data_pc, byte_offset)? as i64,
        )),
        _ => Err(WinedroidError::new(format!(
            "DEX fill-array-data element width {element_width} is not supported"
        ))),
    }
}

fn read_payload_u8(code: &DexCodeItem, data_pc: usize, byte_offset: usize) -> Result<u8> {
    let code_unit = read_code_unit(code, data_pc + byte_offset / 2)?;

    if byte_offset % 2 == 0 {
        Ok((code_unit & 0x00ff) as u8)
    } else {
        Ok((code_unit >> 8) as u8)
    }
}

fn read_payload_u16(code: &DexCodeItem, data_pc: usize, byte_offset: usize) -> Result<u16> {
    let low = u16::from(read_payload_u8(code, data_pc, byte_offset)?);
    let high = u16::from(read_payload_u8(code, data_pc, byte_offset + 1)?);

    Ok(low | (high << 8))
}

fn read_payload_u32(code: &DexCodeItem, data_pc: usize, byte_offset: usize) -> Result<u32> {
    let low = u32::from(read_payload_u16(code, data_pc, byte_offset)?);
    let high = u32::from(read_payload_u16(code, data_pc, byte_offset + 2)?);

    Ok(low | (high << 16))
}

fn read_payload_u64(code: &DexCodeItem, data_pc: usize, byte_offset: usize) -> Result<u64> {
    let low = u64::from(read_payload_u32(code, data_pc, byte_offset)?);
    let high = u64::from(read_payload_u32(code, data_pc, byte_offset + 4)?);

    Ok(low | (high << 32))
}

fn read_i32(code: &DexCodeItem, pc: usize) -> Result<i32> {
    let low = u32::from(read_code_unit(code, pc)?);
    let high = u32::from(read_code_unit(code, pc + 1)?);

    Ok((low | (high << 16)) as i32)
}

fn read_u32(code: &DexCodeItem, pc: usize) -> Result<u32> {
    let low = u32::from(read_code_unit(code, pc)?);
    let high = u32::from(read_code_unit(code, pc + 1)?);

    Ok(low | (high << 16))
}

fn read_static_field_access(
    code: &DexCodeItem,
    pc: usize,
    instruction: u16,
) -> Result<DexStaticFieldAccess> {
    let destination_register = u16::from(high_byte(instruction));
    let field_index = u32::from(read_next_code_unit(code, pc)?);

    Ok(DexStaticFieldAccess {
        field_index,
        destination_register,
    })
}

fn read_static_field_write(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexStaticFieldWrite> {
    let value_register = u16::from(high_byte(instruction));
    let field_index = u32::from(read_next_code_unit(code, pc)?);
    let value = frame.read_value(value_register)?;

    Ok(DexStaticFieldWrite {
        field_index,
        value_register,
        value,
    })
}

fn read_string_access(code: &DexCodeItem, pc: usize, instruction: u16) -> Result<DexStringAccess> {
    Ok(DexStringAccess {
        string_index: u32::from(read_next_code_unit(code, pc)?),
        destination_register: u16::from(high_byte(instruction)),
    })
}

fn read_jumbo_string_access(
    code: &DexCodeItem,
    pc: usize,
    instruction: u16,
) -> Result<DexStringAccess> {
    let low = u32::from(read_next_code_unit(code, pc)?);
    let high = u32::from(read_code_unit(code, pc + 2)?);

    Ok(DexStringAccess {
        string_index: low | (high << 16),
        destination_register: u16::from(high_byte(instruction)),
    })
}

fn read_new_instance(code: &DexCodeItem, pc: usize, instruction: u16) -> Result<DexNewInstance> {
    Ok(DexNewInstance {
        type_index: u32::from(read_next_code_unit(code, pc)?),
        destination_register: u16::from(high_byte(instruction)),
    })
}

fn read_new_array(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexNewArray> {
    let destination_register = u16::from(nibble_a(instruction));
    let size_register = u16::from(nibble_b(instruction));
    let size = frame.read_int(size_register)?;

    if size < 0 {
        return Err(WinedroidError::new(format!(
            "DEX new-array size is negative\nsize register: v{size_register}\nsize: {size}"
        )));
    }

    Ok(DexNewArray {
        type_index: u32::from(read_next_code_unit(code, pc)?),
        destination_register,
        size_register,
        size,
    })
}

fn read_filled_new_array_range(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexFilledNewArray> {
    let value_count = u16::from(high_byte(instruction));
    let type_index = u32::from(read_next_code_unit(code, pc)?);
    let first_register = read_code_unit(code, pc + 2)?;
    let mut argument_registers = Vec::with_capacity(value_count as usize);
    let mut values = Vec::with_capacity(value_count as usize);

    for offset in 0..value_count {
        let register = first_register.checked_add(offset).ok_or_else(|| {
            WinedroidError::new(format!(
                "filled-new-array/range register range overflows\nfirst register: v{first_register}\nvalue count: {value_count}"
            ))
        })?;

        argument_registers.push(register);
        values.push(frame.read_value(register)?);
    }

    Ok(DexFilledNewArray {
        type_index,
        first_register,
        argument_registers,
        values,
    })
}

fn read_filled_new_array(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexFilledNewArray> {
    let value_count = ((instruction >> 12) & 0x0f) as usize;
    let type_index = u32::from(read_next_code_unit(code, pc)?);
    let register_word = read_code_unit(code, pc + 2)?;
    let candidates = [
        register_word & 0x000f,
        (register_word >> 4) & 0x000f,
        (register_word >> 8) & 0x000f,
        (register_word >> 12) & 0x000f,
        (instruction >> 8) & 0x000f,
    ];
    let mut argument_registers = Vec::with_capacity(value_count);
    let mut values = Vec::with_capacity(value_count);

    for register in candidates.iter().take(value_count) {
        argument_registers.push(*register);
        values.push(frame.read_value(*register)?);
    }

    Ok(DexFilledNewArray {
        type_index,
        first_register: *argument_registers.first().unwrap_or(&0),
        argument_registers,
        values,
    })
}

fn read_class_access(code: &DexCodeItem, pc: usize, instruction: u16) -> Result<DexClassAccess> {
    Ok(DexClassAccess {
        type_index: u32::from(read_next_code_unit(code, pc)?),
        destination_register: u16::from(high_byte(instruction)),
    })
}

fn read_type_check(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexTypeCheck> {
    let register = u16::from(high_byte(instruction));

    Ok(DexTypeCheck {
        type_index: u32::from(read_next_code_unit(code, pc)?),
        register,
        value: frame.read_value(register)?,
    })
}

fn read_instance_of(
    code: &DexCodeItem,
    frame: &DexFrame,
    pc: usize,
    instruction: u16,
) -> Result<DexInstanceOf> {
    let value_register = u16::from(nibble_b(instruction));

    Ok(DexInstanceOf {
        type_index: u32::from(read_next_code_unit(code, pc)?),
        destination_register: u16::from(nibble_a(instruction)),
        value_register,
        value: frame.read_value(value_register)?,
    })
}

fn read_five_register_invocation_frame<'a>(
    code: &DexCodeItem,
    frame: &'a DexFrame,
    pc: usize,
    instruction: u16,
    opcode: u8,
) -> Result<DexInvocationFrame<'a>> {
    let argument_count = ((instruction >> 12) & 0x0f) as usize;

    if argument_count > 5 {
        return Err(WinedroidError::new(format!(
            "DEX invoke argument count {argument_count} is too large for format 35c"
        )));
    }

    let method_index = u32::from(read_next_code_unit(code, pc)?);
    let register_word = read_code_unit(code, pc + 2)?;
    let candidates = [
        register_word & 0x000f,
        (register_word >> 4) & 0x000f,
        (register_word >> 8) & 0x000f,
        (register_word >> 12) & 0x000f,
        (instruction >> 8) & 0x000f,
    ];
    let mut argument_registers = DexInvocationRegisters::with_capacity(argument_count);

    for register in candidates.iter().take(argument_count) {
        argument_registers.push(*register);
    }

    Ok(DexInvocationFrame {
        kind: five_register_invoke_kind(opcode)?,
        range: false,
        method_index,
        argument_registers,
        frame,
    })
}

fn read_range_invocation_frame<'a>(
    code: &DexCodeItem,
    frame: &'a DexFrame,
    pc: usize,
    instruction: u16,
    opcode: u8,
) -> Result<DexInvocationFrame<'a>> {
    let argument_count = u16::from(high_byte(instruction));
    let method_index = u32::from(read_next_code_unit(code, pc)?);
    let first_argument_register = read_code_unit(code, pc + 2)?;
    let mut argument_registers = DexInvocationRegisters::with_capacity(argument_count as usize);

    for offset in 0..argument_count {
        let register = first_argument_register
            .checked_add(offset)
            .ok_or_else(|| WinedroidError::new("DEX invoke/range argument register overflow"))?;
        argument_registers.push(register);
    }

    Ok(DexInvocationFrame {
        kind: range_invoke_kind(opcode)?,
        range: true,
        method_index,
        argument_registers,
        frame,
    })
}

fn five_register_invoke_kind(opcode: u8) -> Result<DexInvokeKind> {
    match opcode {
        0x6e => Ok(DexInvokeKind::Virtual),
        0x6f => Ok(DexInvokeKind::Super),
        0x70 => Ok(DexInvokeKind::Direct),
        0x71 => Ok(DexInvokeKind::Static),
        0x72 => Ok(DexInvokeKind::Interface),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an invoke opcode"
        ))),
    }
}

fn range_invoke_kind(opcode: u8) -> Result<DexInvokeKind> {
    match opcode {
        0x74 => Ok(DexInvokeKind::Virtual),
        0x75 => Ok(DexInvokeKind::Super),
        0x76 => Ok(DexInvokeKind::Direct),
        0x77 => Ok(DexInvokeKind::Static),
        0x78 => Ok(DexInvokeKind::Interface),
        _ => Err(WinedroidError::new(format!(
            "opcode 0x{opcode:02x} is not an invoke/range opcode"
        ))),
    }
}

fn initialize_parameter_registers(
    frame: &mut DexFrame,
    code: &DexCodeItem,
    initial_arguments: Option<&[DexValue]>,
) -> Result<()> {
    if code.ins_size == 0 {
        return Ok(());
    }

    if code.ins_size > code.registers_size {
        return Err(WinedroidError::new(format!(
            "DEX code item has {} input register(s) but only {} total register(s)",
            code.ins_size, code.registers_size
        )));
    }

    let first_parameter = code.registers_size - code.ins_size;

    if let Some(arguments) = initial_arguments {
        if arguments.len() != code.ins_size as usize {
            return Err(WinedroidError::new(format!(
                "DEX method expects {} input register(s), got {} argument value(s)",
                code.ins_size,
                arguments.len()
            )));
        }

        for (index, value) in arguments.iter().enumerate() {
            let register = first_parameter + index as u16;
            frame.set_register_from_value(register, value)?;
        }

        return Ok(());
    }

    for index in 0..code.ins_size {
        let register = first_parameter + index;
        frame.set_register(
            register,
            DexValue::Reference(std::rc::Rc::from(format!("p{index}"))),
        )?;
    }

    Ok(())
}

pub(crate) fn frame_from_invocation_arguments(
    code: &DexCodeItem,
    invocation: &DexInvocationFrame<'_>,
) -> Result<DexFrame> {
    if invocation.argument_count() != code.ins_size as usize {
        return Err(WinedroidError::new(format!(
            "DEX method expects {} input register(s), got {} argument value(s)",
            code.ins_size,
            invocation.argument_count()
        )));
    }
    if code.ins_size > code.registers_size {
        return Err(WinedroidError::new(format!(
            "DEX code item has {} input register(s) but only {} total register(s)",
            code.ins_size, code.registers_size
        )));
    }

    let mut frame = DexFrame::new(code.registers_size);
    let first_parameter = code.registers_size - code.ins_size;
    for argument_index in 0..code.ins_size as usize {
        let destination_register = first_parameter + argument_index as u16;
        invocation.copy_argument_to_frame(argument_index, &mut frame, destination_register)?;
    }

    Ok(frame)
}

fn read_next_code_unit(code: &DexCodeItem, pc: usize) -> Result<u16> {
    read_code_unit(code, pc + 1)
}

fn read_code_unit(code: &DexCodeItem, pc: usize) -> Result<u16> {
    code.instructions.get(pc).copied().ok_or_else(|| {
        WinedroidError::new(format!(
            "DEX instruction at {pc} extends past the end of the method"
        ))
    })
}

fn require_code_range(
    code: &DexCodeItem,
    pc: usize,
    code_units: usize,
    description: &str,
) -> Result<()> {
    let Some(end) = pc.checked_add(code_units) else {
        return Err(WinedroidError::new(format!(
            "DEX {description} range is too large"
        )));
    };

    if end > code.instructions.len() {
        return Err(WinedroidError::new(format!(
            "DEX {description} at {pc} extends past the end of the method"
        )));
    }

    Ok(())
}

fn high_byte(instruction: u16) -> u8 {
    (instruction >> 8) as u8
}

fn nibble_a(instruction: u16) -> u8 {
    ((instruction >> 8) & 0x0f) as u8
}

fn nibble_b(instruction: u16) -> u8 {
    ((instruction >> 12) & 0x0f) as u8
}

fn sign_extend_4(value: u8) -> i32 {
    let value = i32::from(value);

    if value & 0x08 != 0 {
        return value | !0x0f;
    }

    value
}

fn opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "nop",
        0x01 => "move",
        0x02 => "move/from16",
        0x03 => "move/16",
        0x04 => "move-wide",
        0x05 => "move-wide/from16",
        0x06 => "move-wide/16",
        0x07 => "move-object",
        0x08 => "move-object/from16",
        0x09 => "move-object/16",
        0x0a => "move-result",
        0x0b => "move-result-wide",
        0x0c => "move-result-object",
        0x0e => "return-void",
        0x0f => "return",
        0x10 => "return-wide",
        0x11 => "return-object",
        0x12 => "const/4",
        0x13 => "const/16",
        0x14 => "const",
        0x15 => "const/high16",
        0x16 => "const-wide/16",
        0x17 => "const-wide/32",
        0x18 => "const-wide",
        0x19 => "const-wide/high16",
        0x1a => "const-string",
        0x1b => "const-string/jumbo",
        0x1c => "const-class",
        0x1d => "monitor-enter",
        0x1e => "monitor-exit",
        0x1f => "check-cast",
        0x20 => "instance-of",
        0x21 => "array-length",
        0x22 => "new-instance",
        0x23 => "new-array",
        0x24 => "filled-new-array",
        0x25 => "filled-new-array/range",
        0x26 => "fill-array-data",
        0x28 => "goto",
        0x29 => "goto/16",
        0x2a => "goto/32",
        0x2b => "packed-switch",
        0x2c => "sparse-switch",
        0x2d => "cmpl-float",
        0x2e => "cmpg-float",
        0x2f => "cmpl-double",
        0x30 => "cmpg-double",
        0x31 => "cmp-long",
        0x32 => "if-eq",
        0x33 => "if-ne",
        0x34 => "if-lt",
        0x35 => "if-ge",
        0x36 => "if-gt",
        0x37 => "if-le",
        0x38 => "if-eqz",
        0x39 => "if-nez",
        0x3a => "if-ltz",
        0x3b => "if-gez",
        0x3c => "if-gtz",
        0x3d => "if-lez",
        0x44 => "aget",
        0x45 => "aget-wide",
        0x46 => "aget-object",
        0x47 => "aget-boolean",
        0x48 => "aget-byte",
        0x49 => "aget-char",
        0x4a => "aget-short",
        0x4b => "aput",
        0x4c => "aput-wide",
        0x4d => "aput-object",
        0x4e => "aput-boolean",
        0x4f => "aput-byte",
        0x50 => "aput-char",
        0x51 => "aput-short",
        0x52 => "iget",
        0x53 => "iget-wide",
        0x54 => "iget-object",
        0x55 => "iget-boolean",
        0x56 => "iget-byte",
        0x57 => "iget-char",
        0x58 => "iget-short",
        0x59 => "iput",
        0x5a => "iput-wide",
        0x5b => "iput-object",
        0x5c => "iput-boolean",
        0x5d => "iput-byte",
        0x5e => "iput-char",
        0x5f => "iput-short",
        0x60 => "sget",
        0x61 => "sget-wide",
        0x62 => "sget-object",
        0x63 => "sget-boolean",
        0x64 => "sget-byte",
        0x65 => "sget-char",
        0x66 => "sget-short",
        0x67 => "sput",
        0x68 => "sput-wide",
        0x69 => "sput-object",
        0x6a => "sput-boolean",
        0x6b => "sput-byte",
        0x6c => "sput-char",
        0x6d => "sput-short",
        0x6e => "invoke-virtual",
        0x6f => "invoke-super",
        0x70 => "invoke-direct",
        0x71 => "invoke-static",
        0x72 => "invoke-interface",
        0x74 => "invoke-virtual/range",
        0x75 => "invoke-super/range",
        0x76 => "invoke-direct/range",
        0x77 => "invoke-static/range",
        0x78 => "invoke-interface/range",
        0x7b => "neg-int",
        0x7c => "not-int",
        0x7d => "neg-long",
        0x7e => "not-long",
        0x7f => "neg-float",
        0x80 => "neg-double",
        0x81 => "int-to-long",
        0x82 => "int-to-float",
        0x83 => "int-to-double",
        0x84 => "long-to-int",
        0x85 => "long-to-float",
        0x86 => "long-to-double",
        0x87 => "float-to-int",
        0x88 => "float-to-long",
        0x89 => "float-to-double",
        0x8a => "double-to-int",
        0x8b => "double-to-long",
        0x8c => "double-to-float",
        0x8d => "int-to-byte",
        0x8e => "int-to-char",
        0x8f => "int-to-short",
        0x90 => "add-int",
        0x91 => "sub-int",
        0x92 => "mul-int",
        0x93 => "div-int",
        0x94 => "rem-int",
        0x95 => "and-int",
        0x96 => "or-int",
        0x97 => "xor-int",
        0x98 => "shl-int",
        0x99 => "shr-int",
        0x9a => "ushr-int",
        0x9b => "add-long",
        0x9c => "sub-long",
        0x9d => "mul-long",
        0x9e => "div-long",
        0x9f => "rem-long",
        0xa0 => "and-long",
        0xa1 => "or-long",
        0xa2 => "xor-long",
        0xa3 => "shl-long",
        0xa4 => "shr-long",
        0xa5 => "ushr-long",
        0xa6 => "add-float",
        0xa7 => "sub-float",
        0xa8 => "mul-float",
        0xa9 => "div-float",
        0xaa => "rem-float",
        0xab => "add-double",
        0xac => "sub-double",
        0xad => "mul-double",
        0xae => "div-double",
        0xaf => "rem-double",
        0xb0 => "add-int/2addr",
        0xb1 => "sub-int/2addr",
        0xb2 => "mul-int/2addr",
        0xb3 => "div-int/2addr",
        0xb4 => "rem-int/2addr",
        0xb5 => "and-int/2addr",
        0xb6 => "or-int/2addr",
        0xb7 => "xor-int/2addr",
        0xb8 => "shl-int/2addr",
        0xb9 => "shr-int/2addr",
        0xba => "ushr-int/2addr",
        0xbb => "add-long/2addr",
        0xbc => "sub-long/2addr",
        0xbd => "mul-long/2addr",
        0xbe => "div-long/2addr",
        0xbf => "rem-long/2addr",
        0xc0 => "and-long/2addr",
        0xc1 => "or-long/2addr",
        0xc2 => "xor-long/2addr",
        0xc3 => "shl-long/2addr",
        0xc4 => "shr-long/2addr",
        0xc5 => "ushr-long/2addr",
        0xc6 => "add-float/2addr",
        0xc7 => "sub-float/2addr",
        0xc8 => "mul-float/2addr",
        0xc9 => "div-float/2addr",
        0xca => "rem-float/2addr",
        0xcb => "add-double/2addr",
        0xcc => "sub-double/2addr",
        0xcd => "mul-double/2addr",
        0xce => "div-double/2addr",
        0xcf => "rem-double/2addr",
        0xd0 => "add-int/lit16",
        0xd1 => "rsub-int",
        0xd2 => "mul-int/lit16",
        0xd3 => "div-int/lit16",
        0xd4 => "rem-int/lit16",
        0xd5 => "and-int/lit16",
        0xd6 => "or-int/lit16",
        0xd7 => "xor-int/lit16",
        0xd8 => "add-int/lit8",
        0xd9 => "rsub-int/lit8",
        0xda => "mul-int/lit8",
        0xdb => "div-int/lit8",
        0xdc => "rem-int/lit8",
        0xdd => "and-int/lit8",
        0xde => "or-int/lit8",
        0xdf => "xor-int/lit8",
        0xe0 => "shl-int/lit8",
        0xe1 => "shr-int/lit8",
        0xe2 => "ushr-int/lit8",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrips_every_value_kind() {
        let mut frame = DexFrame::new(8);
        let object = DexValue::Object(DexObjectRef {
            id: 42,
            class_descriptor: std::rc::Rc::from("Lcom/Foo;"),
        });
        let values = [
            DexValue::Null,
            DexValue::Int(-7),
            DexValue::Long(0x0123_4567_89ab_cdef),
            DexValue::Float(3.5),
            DexValue::String(std::rc::Rc::from("hi")),
            DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/x")),
            object,
        ];
        for (index, value) in values.iter().enumerate() {
            frame.set_register(index as u16, value.clone()).unwrap();
        }
        for (index, value) in values.iter().enumerate() {
            assert_eq!(&frame.read_value(index as u16).unwrap(), value);
        }
        assert_eq!(frame.read_int(1).unwrap(), -7);
        assert_eq!(frame.read_long(2).unwrap(), 0x0123_4567_89ab_cdef);
        assert_eq!(frame.read_float(3).unwrap(), 3.5);
        assert!(frame.is_zero(0).unwrap());
        assert!(!frame.is_zero(1).unwrap());
        assert!(frame.read_value(7).is_err());
    }

    #[test]
    fn from_values_and_clone_preserve_boxed_semantics() {
        let payload = std::rc::Rc::<str>::from("Lx;");
        let frame = DexFrame::from_values(vec![
            DexValue::Int(9),
            DexValue::Object(DexObjectRef {
                id: 3,
                class_descriptor: std::rc::Rc::clone(&payload),
            }),
        ]);
        let cloned = frame.clone();
        assert_eq!(frame, cloned);
        assert_eq!(frame.read_int(0).unwrap(), 9);
        assert_eq!(
            cloned.read_value(1).unwrap(),
            DexValue::Object(DexObjectRef {
                id: 3,
                class_descriptor: std::rc::Rc::clone(&payload),
            })
        );
    }

    #[test]
    fn invocation_frame_materializes_argument_values_on_demand() {
        let payload = std::rc::Rc::<str>::from("Lexample/Foo;");
        let mut frame = DexFrame::new(3);
        frame.set_int(0, 7).unwrap();
        frame
            .set_register(
                2,
                DexValue::Object(DexObjectRef {
                    id: 88,
                    class_descriptor: std::rc::Rc::clone(&payload),
                }),
            )
            .unwrap();
        let mut registers = DexInvocationRegisters::new();
        registers.push(2);
        registers.push(0);
        let invocation = DexInvocationFrame {
            kind: DexInvokeKind::Virtual,
            range: false,
            method_index: 12,
            argument_registers: registers,
            frame: &frame,
        };

        assert_eq!(
            invocation.object_argument_class_descriptor(0).unwrap(),
            Some("Lexample/Foo;")
        );
        let materialized = invocation.materialize().unwrap();
        assert_eq!(materialized.kind, DexInvokeKind::Virtual);
        assert_eq!(materialized.method_index, 12);
        assert_eq!(materialized.argument_registers.as_slice(), &[2, 0]);
        assert_eq!(materialized.arguments.len(), 2);
        assert_eq!(materialized.arguments[1], DexValue::Int(7));
    }

    #[test]
    fn invocation_frame_reads_argument_class_descriptors_without_materializing() {
        let object_descriptor = std::rc::Rc::<str>::from("Lexample/Foo;");
        let reference_descriptor = std::rc::Rc::<str>::from("Lexample/Bar;:static");
        let mut frame = DexFrame::new(4);
        frame
            .set_register(
                0,
                DexValue::Object(DexObjectRef {
                    id: 7,
                    class_descriptor: std::rc::Rc::clone(&object_descriptor),
                }),
            )
            .unwrap();
        frame
            .set_register(1, DexValue::Reference(reference_descriptor))
            .unwrap();
        frame
            .set_register(2, DexValue::String(std::rc::Rc::from("text")))
            .unwrap();
        frame.set_register(3, DexValue::Null).unwrap();
        let mut registers = DexInvocationRegisters::new();
        registers.push(0);
        registers.push(1);
        registers.push(2);
        registers.push(3);
        let invocation = DexInvocationFrame {
            kind: DexInvokeKind::Virtual,
            range: false,
            method_index: 99,
            argument_registers: registers,
            frame: &frame,
        };

        assert_eq!(
            invocation.argument_class_descriptor(0).unwrap(),
            Some("Lexample/Foo;")
        );
        assert_eq!(
            invocation.argument_class_descriptor(1).unwrap(),
            Some("Lexample/Bar;")
        );
        assert_eq!(
            invocation.argument_class_descriptor(2).unwrap(),
            Some("Ljava/lang/String;")
        );
        assert_eq!(invocation.argument_class_descriptor(3).unwrap(), None);
        assert_eq!(invocation.argument_class_descriptor(4).unwrap(), None);
    }

    #[test]
    fn invocation_frame_copies_arguments_into_callee_parameter_window() {
        let payload = std::rc::Rc::<str>::from("Lexample/Foo;");
        let mut caller = DexFrame::new(3);
        caller.set_int(1, 41).unwrap();
        caller
            .set_register(
                0,
                DexValue::Object(DexObjectRef {
                    id: 9,
                    class_descriptor: std::rc::Rc::clone(&payload),
                }),
            )
            .unwrap();
        assert_eq!(std::rc::Rc::strong_count(&payload), 2);

        let mut registers = DexInvocationRegisters::new();
        registers.push(1);
        registers.push(0);
        let invocation = DexInvocationFrame {
            kind: DexInvokeKind::Direct,
            range: false,
            method_index: 44,
            argument_registers: registers,
            frame: &caller,
        };
        let mut code = code_item(4, vec![0x000e]);
        code.ins_size = 2;

        let callee = frame_from_invocation_arguments(&code, &invocation).unwrap();

        assert_eq!(std::rc::Rc::strong_count(&payload), 3);
        assert!(callee.read_value(0).is_err());
        assert!(callee.read_value(1).is_err());
        assert_eq!(callee.read_int(2).unwrap(), 41);
        assert_eq!(
            callee.read_value(3).unwrap(),
            DexValue::Object(DexObjectRef {
                id: 9,
                class_descriptor: std::rc::Rc::clone(&payload),
            })
        );
    }

    #[test]
    fn art_shadow_frame_pending_result_moves_object_raw_value_into_register() {
        let class_descriptor = std::rc::Rc::<str>::from("Lexample/Foo;");
        let result = DexPendingResult::from_value(DexValue::Object(DexObjectRef {
            id: 42,
            class_descriptor: std::rc::Rc::clone(&class_descriptor),
        }));
        let mut frame = DexFrame::new(2);

        result.write_to_frame(&mut frame, 1).unwrap();

        assert_eq!(
            frame.read_value(1).unwrap(),
            DexValue::Object(DexObjectRef {
                id: 42,
                class_descriptor: std::rc::Rc::clone(&class_descriptor),
            })
        );
        assert!(frame.read_value(0).is_err());
    }

    #[test]
    fn art_shadow_frame_pending_result_moves_wide_raw_value_into_register_pair() {
        let result = DexPendingResult::from_value(DexValue::Long(0x1122_3344_5566_7788));
        let mut frame = DexFrame::new(3);

        result.write_wide_to_frame(&mut frame, 1).unwrap();

        assert_eq!(frame.read_long(1).unwrap(), 0x1122_3344_5566_7788);
        assert_eq!(frame.read_long(2).unwrap(), 0x1122_3344_5566_7788);
        assert!(frame.read_value(0).is_err());
    }

    #[test]
    fn art_shadow_frame_move_result_wide_rejects_narrow_pending_result() {
        let result = DexPendingResult::from_value(DexValue::Int(7));
        let mut frame = DexFrame::new(2);

        let error = result.write_wide_to_frame(&mut frame, 0).unwrap_err();

        assert!(error
            .to_string()
            .contains("move-result-wide expected a wide pending result"));
        assert!(frame.read_value(0).is_err());
        assert!(frame.read_value(1).is_err());
    }

    // cite: art shadow_frame.h#SetVReg, SetVRegReference
    #[test]
    fn zeroed_frame_places_arguments_in_the_parameter_window() {
        let class_descriptor = std::rc::Rc::<str>::from("Lexample/Foo;");
        let object = DexValue::Object(DexObjectRef {
            id: 17,
            class_descriptor: std::rc::Rc::clone(&class_descriptor),
        });
        let frame = DexFrame::from_zeroed_registers_and_arguments(
            5,
            2,
            Some(&[DexValue::Int(11), object.clone()]),
        )
        .unwrap();

        assert_eq!(frame.read_int(0).unwrap(), 0);
        assert_eq!(frame.read_int(1).unwrap(), 0);
        assert_eq!(frame.read_int(2).unwrap(), 0);
        assert_eq!(frame.read_int(3).unwrap(), 11);
        assert_eq!(frame.read_value(4).unwrap(), object);
        assert_eq!(std::rc::Rc::strong_count(&class_descriptor), 3);
    }

    // cite: art shadow_frame.h#GetVReg
    #[test]
    fn raw_frame_materializes_deopt_windows() {
        let frame = DexFrame::from_values(vec![
            DexValue::Int(1),
            DexValue::Int(2),
            DexValue::Int(3),
            DexValue::Int(4),
        ]);

        assert_eq!(
            frame.read_values_window(1, 2).unwrap(),
            vec![DexValue::Int(2), DexValue::Int(3)]
        );
        assert!(frame.read_values_window(8, 2).unwrap().is_empty());
    }

    #[test]
    fn zeroed_argument_frame_rejects_mismatched_input_count() {
        let error = DexFrame::from_zeroed_registers_and_arguments(3, 2, Some(&[DexValue::Int(1)]))
            .expect_err("argument count should be checked");

        assert!(error
            .to_string()
            .contains("DEX method expects 2 input register(s)"));
    }

    #[test]
    fn executes_return_void() {
        let code = code_item(0, vec![0x000e]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Void
        );
    }

    #[test]
    fn executes_return_wide() {
        let mut code = code_item(2, vec![0x0010]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(42), DexValue::Long(42)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(42))
        );
    }

    #[test]
    fn executes_return_float() {
        let mut code = code_item(1, vec![0x000f]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(1.5)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Float(1.5))
        );
    }

    #[test]
    fn executes_const4_and_return_int() {
        let code = code_item(1, vec![0x7012, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(7)
        );
    }

    #[test]
    fn executes_const16_and_return_int() {
        let code = code_item(1, vec![0x0013, 0xfffe, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-2)
        );
    }

    #[test]
    fn executes_const_and_return_int() {
        let code = code_item(1, vec![0x0014, 0x5678, 0x1234, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(0x1234_5678)
        );
    }

    #[test]
    fn executes_const_high16_and_return_int() {
        let code = code_item(1, vec![0x0015, 0xffff, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-65536)
        );
    }

    #[test]
    fn executes_const_wide16_and_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0016, 0xfffe, 0x0111]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(-2))
        );
    }

    #[test]
    fn executes_const_wide32_and_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0017, 0xfffe, 0xffff, 0x0111]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(-2))
        );
    }

    #[test]
    fn executes_const_wide_and_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0018, 0x7788, 0x5566, 0x3344, 0x1122, 0x0111]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(0x1122_3344_5566_7788))
        );
    }

    #[test]
    fn executes_const_wide_high16_and_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0019, 0x3ff0, 0x0111]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(0x3ff0_0000_0000_0000))
        );
    }

    #[test]
    fn executes_cmp_long() {
        let mut code = code_item(5, vec![0x0031, 0x0301, 0x000f]);
        code.ins_size = 4;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::Long(7),
                    DexValue::Long(7),
                    DexValue::Long(5),
                    DexValue::Long(5),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_move_wide_from16() {
        let mut code = code_item(4, vec![0x0005, 0x0002, 0x0011]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(42), DexValue::Long(42)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(42))
        );
    }

    #[test]
    fn move_wide_from16_initializes_high_register_slot() {
        let mut code = code_item(4, vec![0x0005, 0x0002, 0x0111]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(42), DexValue::Long(42)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(42))
        );
    }

    #[test]
    fn executes_xor_int_lit8() {
        let code = code_item(1, vec![0x5012, 0x00df, 0x0f00, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(0x0a)
        );
    }

    #[test]
    fn executes_and_int_lit16() {
        let code = code_item(1, vec![0x5012, 0x00d5, 0x0003, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_add_int() {
        let code = code_item(3, vec![0x2112, 0x3212, 0x0090, 0x0201, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(5)
        );
    }

    #[test]
    fn executes_shl_int() {
        let code = code_item(3, vec![0x2112, 0x3212, 0x0098, 0x0201, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(16)
        );
    }

    #[test]
    fn executes_shr_int() {
        let code = code_item(3, vec![0xc112, 0x1212, 0x0099, 0x0201, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-2)
        );
    }

    #[test]
    fn executes_ushr_int() {
        let code = code_item(3, vec![0xf112, 0x1212, 0x009a, 0x0201, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(0x7fff_ffff)
        );
    }

    #[test]
    fn executes_add_int_2addr() {
        let code = code_item(2, vec![0x7012, 0x5112, 0x10b0, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(12)
        );
    }

    #[test]
    fn executes_sub_int_2addr() {
        let code = code_item(2, vec![0x7012, 0x5112, 0x10b1, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(2)
        );
    }

    #[test]
    fn executes_add_long_2addr() {
        let code = code_item(4, vec![0x0016, 0x0007, 0x0216, 0x0005, 0x20bb, 0x0010]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12))
        );
    }

    #[test]
    fn executes_and_int_2addr() {
        let code = code_item(2, vec![0x5012, 0x3112, 0x10b5, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_or_int_2addr() {
        let code = code_item(2, vec![0x5012, 0x3112, 0x10b6, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(7)
        );
    }

    #[test]
    fn executes_shl_int_2addr() {
        let code = code_item(2, vec![0x2012, 0x3112, 0x10b8, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(16)
        );
    }

    #[test]
    fn executes_neg_int() {
        let code = code_item(2, vec![0x5112, 0x107b, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-5)
        );
    }

    #[test]
    fn executes_not_int() {
        let code = code_item(2, vec![0x5112, 0x107c, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-6)
        );
    }

    #[test]
    fn executes_neg_long() {
        let mut code = code_item(3, vec![0x107d, 0x0011]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(5), DexValue::Long(5)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(-5))
        );
    }

    #[test]
    fn executes_not_long_and_initializes_high_register_slot() {
        let mut code = code_item(3, vec![0x107e, 0x0111]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(5), DexValue::Long(5)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(!5))
        );
    }

    #[test]
    fn executes_neg_float() {
        let mut code = code_item(2, vec![0x107f, 0x0087, 0x000f]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(3.5)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-3)
        );
    }

    #[test]
    fn executes_rsub_int_lit16() {
        let code = code_item(1, vec![0x2012, 0x00d1, 0x000a, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(8)
        );
    }

    #[test]
    fn executes_ushr_int_lit8() {
        let code = code_item(1, vec![0xf012, 0x00e2, 0x0100, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(0x7fff_ffff)
        );
    }

    #[test]
    fn executes_float_to_int() {
        let mut code = code_item(2, vec![0x1087, 0x000f]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(12.75)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(12)
        );
    }

    #[test]
    fn executes_int_to_float() {
        let mut code = code_item(2, vec![0x1082, 0x0087, 0x000f]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(23)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(23)
        );
    }

    #[test]
    fn executes_int_to_double() {
        let mut code = code_item(3, vec![0x2083, 0x0011]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(12)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12.0_f64.to_bits() as i64))
        );
    }

    #[test]
    fn int_to_double_initializes_high_register_slot() {
        let mut code = code_item(3, vec![0x2083, 0x0111]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(12)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12.0_f64.to_bits() as i64))
        );
    }

    #[test]
    fn executes_long_to_int() {
        let mut code = code_item(3, vec![0x1084, 0x000f]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(23), DexValue::Long(23)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(23)
        );
    }

    #[test]
    fn executes_long_to_float_and_double() {
        let mut long_to_float = code_item(3, vec![0x1085, 0x0087, 0x000f]);
        long_to_float.ins_size = 2;
        let mut long_to_double = code_item(3, vec![0x1086, 0x0010]);
        long_to_double.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &long_to_float,
                Some(&[DexValue::Long(23), DexValue::Long(23)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(23)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &long_to_double,
                Some(&[DexValue::Long(23), DexValue::Long(23)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(23.0_f64.to_bits() as i64))
        );
    }

    #[test]
    fn executes_float_to_long_and_double() {
        let mut float_to_long = code_item(2, vec![0x1088, 0x0010]);
        float_to_long.ins_size = 1;
        let mut float_to_double = code_item(2, vec![0x1089, 0x0010]);
        float_to_double.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &float_to_long,
                Some(&[DexValue::Float(12.75)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12))
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &float_to_double,
                Some(&[DexValue::Float(12.5)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12.5_f64.to_bits() as i64))
        );
    }

    #[test]
    fn executes_double_to_int_and_long() {
        let mut double_to_int = code_item(3, vec![0x108a, 0x000f]);
        double_to_int.ins_size = 2;
        let mut double_to_long = code_item(3, vec![0x108b, 0x0010]);
        double_to_long.ins_size = 2;
        let value = 12.75_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &double_to_int,
                Some(&[DexValue::Long(value), DexValue::Long(value)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(12)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &double_to_long,
                Some(&[DexValue::Long(value), DexValue::Long(value)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12))
        );
    }

    #[test]
    fn executes_int_narrowing_conversions() {
        let mut int_to_byte = code_item(2, vec![0x108d, 0x000f]);
        int_to_byte.ins_size = 1;
        let mut int_to_char = code_item(2, vec![0x108e, 0x000f]);
        int_to_char.ins_size = 1;
        let mut int_to_short = code_item(2, vec![0x108f, 0x000f]);
        int_to_short.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &int_to_byte,
                Some(&[DexValue::Int(0x0000_00ff)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-1)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &int_to_char,
                Some(&[DexValue::Int(-1)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(0xffff)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &int_to_short,
                Some(&[DexValue::Int(0x0000_8001)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-32767)
        );
    }

    #[test]
    fn executes_double_to_float_with_raw_long_bits() {
        let mut code = code_item(3, vec![0x108c, 0x0087, 0x000f]);
        code.ins_size = 2;
        let value = 23.0_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(value), DexValue::Long(value)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(23)
        );
    }

    #[test]
    fn executes_neg_double_with_raw_long_bits() {
        let mut code = code_item(3, vec![0x1080, 0x0011]);
        code.ins_size = 2;
        let value = 3.5_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(value), DexValue::Long(value)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long((-3.5_f64).to_bits() as i64))
        );
    }

    #[test]
    fn neg_double_initializes_high_register_slot() {
        let mut code = code_item(3, vec![0x1080, 0x0111]);
        code.ins_size = 2;
        let value = 3.5_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(value), DexValue::Long(value)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long((-3.5_f64).to_bits() as i64))
        );
    }

    #[test]
    fn executes_cmpl_float() {
        let mut code = code_item(4, vec![0x002d, 0x0302, 0x000f]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(2.0), DexValue::Float(3.0)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-1)
        );
    }

    #[test]
    fn float_compare_uses_nan_bias() {
        let mut cmpl = code_item(4, vec![0x002d, 0x0302, 0x000f]);
        cmpl.ins_size = 2;
        let mut cmpg = code_item(4, vec![0x002e, 0x0302, 0x000f]);
        cmpg.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &cmpl,
                Some(&[DexValue::Float(f32::NAN), DexValue::Float(3.0)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-1)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &cmpg,
                Some(&[DexValue::Float(f32::NAN), DexValue::Float(3.0)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_cmpl_double() {
        let mut code = code_item(5, vec![0x002f, 0x0301, 0x000f]);
        code.ins_size = 4;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::Long(2.0_f64.to_bits() as i64),
                    DexValue::Long(2.0_f64.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-1)
        );
    }

    #[test]
    fn double_compare_uses_nan_bias() {
        let mut cmpl = code_item(5, vec![0x002f, 0x0301, 0x000f]);
        cmpl.ins_size = 4;
        let mut cmpg = code_item(5, vec![0x0030, 0x0301, 0x000f]);
        cmpg.ins_size = 4;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &cmpl,
                Some(&[
                    DexValue::Long(f64::NAN.to_bits() as i64),
                    DexValue::Long(f64::NAN.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(-1)
        );
        assert_eq!(
            interpret_code_with_initial_arguments(
                &cmpg,
                Some(&[
                    DexValue::Long(f64::NAN.to_bits() as i64),
                    DexValue::Long(f64::NAN.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                    DexValue::Long(3.0_f64.to_bits() as i64),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_div_float_2addr() {
        let mut code = code_item(3, vec![0x21c9, 0x1087, 0x000f]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(24.0), DexValue::Float(3.0)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(8)
        );
    }

    #[test]
    fn executes_mul_float() {
        let mut code = code_item(4, vec![0x00a8, 0x0302, 0x0087, 0x000f]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Float(6.0), DexValue::Float(7.0)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(42)
        );
    }

    #[test]
    fn executes_mul_double() {
        let mut code = code_item(6, vec![0x00ad, 0x0402, 0x0010]);
        code.ins_size = 4;
        let left = 6.0_f64.to_bits() as i64;
        let right = 7.0_f64.to_bits() as i64;
        let expected = 42.0_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::Long(left),
                    DexValue::Long(left),
                    DexValue::Long(right),
                    DexValue::Long(right),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(expected))
        );
    }

    #[test]
    fn executes_div_float_2addr_with_const_high16_bits() {
        let mut code = code_item(2, vec![0x1182, 0x0015, 0x3f40, 0x01c9, 0x1087, 0x000f]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(3)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Int(4)
        );
    }

    #[test]
    fn executes_div_double_2addr_with_raw_long_bits() {
        let mut code = code_item(4, vec![0x20ce, 0x0010]);
        code.ins_size = 4;
        let left = 24.0_f64.to_bits() as i64;
        let right = 3.0_f64.to_bits() as i64;
        let expected = 8.0_f64.to_bits() as i64;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::Long(left),
                    DexValue::Long(left),
                    DexValue::Long(right),
                    DexValue::Long(right),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(expected))
        );
    }

    #[test]
    fn executes_shl_long_2addr_with_int_shift_count() {
        let mut code = code_item(3, vec![0x20c3, 0x0010]);
        code.ins_size = 3;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(2), DexValue::Long(2), DexValue::Int(3)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(16))
        );
    }

    #[test]
    fn executes_int_to_long() {
        let mut code = code_item(3, vec![0x2081, 0x0011]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(12)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12))
        );
    }

    #[test]
    fn int_to_long_initializes_high_register_slot() {
        let mut code = code_item(3, vec![0x2081, 0x0111]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Int(12)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(12))
        );
    }

    #[test]
    fn executes_sub_long() {
        let mut code = code_item(6, vec![0x009c, 0x0504, 0x0011]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(20), DexValue::Long(4)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(16))
        );
    }

    #[test]
    fn executes_div_long() {
        let mut code = code_item(6, vec![0x009e, 0x0504, 0x0011]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(20), DexValue::Long(4)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(5))
        );
    }

    #[test]
    fn div_long_initializes_high_register_slot() {
        let mut code = code_item(6, vec![0x009e, 0x0504, 0x0111]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(20), DexValue::Long(4)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(5))
        );
    }

    #[test]
    fn executes_ushr_long() {
        let mut code = code_item(6, vec![0x00a5, 0x0504, 0x0011]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Long(-1), DexValue::Int(1)]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(0x7fff_ffff_ffff_ffff))
        );
    }

    #[test]
    fn div_long_by_zero_errors() {
        let mut code = code_item(6, vec![0x009e, 0x0504, 0x0011]);
        code.ins_size = 2;

        let error = interpret_code_with_initial_arguments(
            &code,
            Some(&[DexValue::Long(20), DexValue::Long(0)]),
            &mut recording_handler(),
        )
        .expect_err("division by zero should fail");

        assert!(
            error.to_string().contains("DEX div-long by zero"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn errors_on_first_unsupported_opcode() {
        let code = code_item(1, vec![0x0073]);
        let error =
            interpret_without_invocations(&code).expect_err("unsupported opcode should fail");

        assert!(
            error.to_string().contains("unsupported DEX opcode 0x73"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn moves_object_parameter_before_next_unsupported_opcode() {
        let mut code = code_item(2, vec![0x0008, 0x0001, 0x0073]);
        code.ins_size = 1;

        let error = interpret_without_invocations(&code).expect_err("invoke should still fail");

        assert!(
            error.to_string().contains("unsupported DEX opcode 0x73"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn decodes_invoke_super_range() {
        let mut code = code_item(3, vec![0x0275, 0x1234, 0x0001, 0x000e]);
        code.ins_size = 2;
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let invocation = handler.invocation.expect("invocation should be recorded");
        assert_eq!(invocation.kind, DexInvokeKind::Super);
        assert!(invocation.range);
        assert_eq!(invocation.method_index, 0x1234);
        assert_eq!(invocation.argument_registers.as_slice(), &[1, 2]);
    }

    #[test]
    fn decodes_invoke_super_five_register_form() {
        let mut code = code_item(5, vec![0x206f, 0x1234, 0x0043, 0x000e]);
        code.ins_size = 2;
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let invocation = handler.invocation.expect("invocation should be recorded");
        assert_eq!(invocation.kind, DexInvokeKind::Super);
        assert!(!invocation.range);
        assert_eq!(invocation.method_index, 0x1234);
        assert_eq!(invocation.argument_registers.as_slice(), &[3, 4]);
    }

    #[test]
    fn decodes_instance_field_read() {
        let mut code = code_item(2, vec![0x1054, 0x1234, 0x000e]);
        code.ins_size = 1;
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let access = handler
            .field_access
            .expect("field access should be recorded");
        assert_eq!(access.field_index, 0x1234);
        assert_eq!(access.destination_register, 0);
        assert_eq!(access.object_register, 1);
    }

    #[test]
    fn decodes_instance_field_read_variants() {
        let mut code = code_item(2, vec![0x1056, 0x1234, 0x000e]);
        code.ins_size = 1;
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let access = handler
            .field_access
            .expect("field access should be recorded");
        assert_eq!(access.field_index, 0x1234);
    }

    #[test]
    fn iget_wide_initializes_high_register_slot() {
        let mut code = code_item(2, vec![0x1053, 0x1234, 0x0111]);
        code.ins_size = 1;
        let mut handler = LongFieldReadHandler;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Reference(std::rc::Rc::from("object"))]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(55))
        );
    }

    #[test]
    fn decodes_instance_field_write() {
        let mut code = code_item(2, vec![0x0012, 0x105b, 0x1234, 0x000e]);
        code.ins_size = 1;
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let write = handler.field_write.expect("field write should be recorded");
        assert_eq!(write.field_index, 0x1234);
        assert_eq!(write.value_register, 0);
        assert_eq!(write.object_register, 1);
    }

    #[test]
    fn decodes_static_field_read() {
        let code = code_item(1, vec![0x0062, 0x1234, 0x000e]);
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let access = handler
            .static_field_access
            .expect("static field access should be recorded");
        assert_eq!(access.field_index, 0x1234);
        assert_eq!(access.destination_register, 0);
    }

    #[test]
    fn sget_wide_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0061, 0x1234, 0x0111]);
        let mut handler = LongFieldReadHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(55))
        );
    }

    #[test]
    fn decodes_static_field_write() {
        let code = code_item(1, vec![0x1012, 0x0069, 0x1234, 0x000e]);
        let mut handler = recording_handler();

        interpret_code_with_handler(&code, &mut handler).expect("code should run");

        let write = handler
            .static_field_write
            .expect("static field write should be recorded");
        assert_eq!(write.field_index, 0x1234);
        assert_eq!(write.value_register, 0);
        assert_eq!(write.value, DexValue::Int(1));
    }

    #[test]
    fn loads_const_string() {
        let code = code_item(1, vec![0x001a, 0x1234, 0x0011]);
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("string#4660")))
        );

        let access = handler
            .string_access
            .expect("string access should be recorded");
        assert_eq!(access.string_index, 0x1234);
        assert_eq!(access.destination_register, 0);
    }

    #[test]
    fn loads_const_string_jumbo() {
        let code = code_item(2, vec![0x011b, 0x5678, 0x1234, 0x0111]);
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("string#305419896")))
        );

        let access = handler
            .string_access
            .expect("string access should be recorded");
        assert_eq!(access.string_index, 0x1234_5678);
        assert_eq!(access.destination_register, 1);
    }

    #[test]
    fn allocates_new_instance() {
        let code = code_item(1, vec![0x0022, 0x1234, 0x0011]);
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(test_object(4660, "type#4660"))
        );

        let instance = handler
            .new_instance
            .expect("new instance should be recorded");
        assert_eq!(instance.type_index, 0x1234);
        assert_eq!(instance.destination_register, 0);
    }

    #[test]
    fn allocates_new_array() {
        let code = code_item(2, vec![0x3112, 0x1023, 0x1234, 0x0011]);
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(test_object(4660, "array#4660"))
        );

        let array = handler.new_array.expect("new array should be recorded");
        assert_eq!(array.type_index, 0x1234);
        assert_eq!(array.destination_register, 0);
        assert_eq!(array.size_register, 1);
        assert_eq!(array.size, 3);
    }

    #[test]
    fn executes_array_length() {
        let mut code = code_item(2, vec![0x1021, 0x000f]);
        code.ins_size = 1;
        let mut handler = recording_handler();
        let array = test_object(1, "[Ljava/lang/Object;");

        assert_eq!(
            interpret_code_with_initial_arguments(&code, Some(&[array.clone()]), &mut handler)
                .expect("code should run"),
            DexExecutionResult::Int(3)
        );

        let length = handler
            .array_length
            .expect("array length should be recorded");
        assert_eq!(length.destination_register, 0);
        assert_eq!(length.array_register, 1);
        assert_eq!(length.array, array);
    }

    #[test]
    fn executes_aget() {
        let mut code = code_item(3, vec![0x0044, 0x0201, 0x000f]);
        code.ins_size = 2;
        let mut handler = recording_handler();
        let array = test_object(1, "[I");

        handler.array_read_value = DexValue::Int(7);

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[array.clone(), DexValue::Int(1)]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Int(7)
        );

        let read = handler.array_read.expect("array read should be recorded");
        assert_eq!(read.destination_register, 0);
        assert_eq!(read.array_register, 1);
        assert_eq!(read.index_register, 2);
        assert_eq!(read.array, array);
        assert_eq!(read.index, 1);
    }

    #[test]
    fn executes_aget_object() {
        let mut code = code_item(3, vec![0x0046, 0x0201, 0x0011]);
        code.ins_size = 2;
        let mut handler = recording_handler();
        let array = test_object(1, "[Ljava/lang/Object;");

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[array.clone(), DexValue::Int(1)]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("array-value")))
        );

        let read = handler.array_read.expect("array read should be recorded");
        assert_eq!(read.destination_register, 0);
        assert_eq!(read.array_register, 1);
        assert_eq!(read.index_register, 2);
        assert_eq!(read.array, array);
        assert_eq!(read.index, 1);
    }

    #[test]
    fn executes_aput_object() {
        let mut code = code_item(4, vec![0x034d, 0x0201, 0x000e]);
        code.ins_size = 3;
        let mut handler = recording_handler();
        let array = test_object(1, "[Ljava/lang/Object;");
        let value = DexValue::String(std::rc::Rc::from("stored"));

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[array.clone(), DexValue::Int(1), value.clone()]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Void
        );

        let write = handler.array_write.expect("array write should be recorded");
        assert_eq!(write.value_register, 3);
        assert_eq!(write.array_register, 1);
        assert_eq!(write.index_register, 2);
        assert_eq!(write.array, array);
        assert_eq!(write.index, 1);
        assert_eq!(write.value, value);
    }

    #[test]
    fn executes_aput_char() {
        let mut code = code_item(4, vec![0x0350, 0x0201, 0x000e]);
        code.ins_size = 3;
        let mut handler = recording_handler();
        let array = test_object(1, "[C");

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[array.clone(), DexValue::Int(1), DexValue::Int(65)]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Void
        );

        let write = handler.array_write.expect("array write should be recorded");
        assert_eq!(write.value_register, 3);
        assert_eq!(write.array_register, 1);
        assert_eq!(write.index_register, 2);
        assert_eq!(write.array, array);
        assert_eq!(write.index, 1);
        assert_eq!(write.value, DexValue::Int(65));
    }

    #[test]
    fn executes_fill_array_data_for_int_payload() {
        let mut code = code_item(
            1,
            vec![
                0x0026, 0x0004, 0x0000, 0x000e, 0x0300, 0x0004, 0x0003, 0x0000, 0x000a, 0x0000,
                0x0014, 0x0000, 0x001e, 0x0000,
            ],
        );
        code.ins_size = 1;
        let mut handler = recording_handler();
        let array = test_object(1, "[I");

        assert_eq!(
            interpret_code_with_initial_arguments(&code, Some(&[array.clone()]), &mut handler)
                .expect("code should run"),
            DexExecutionResult::Void
        );

        assert_eq!(handler.array_writes.len(), 3);
        for (index, expected_value) in [10, 20, 30].into_iter().enumerate() {
            let write = &handler.array_writes[index];
            assert_eq!(write.array_register, 0);
            assert_eq!(write.array, array);
            assert_eq!(write.index, index as i32);
            assert_eq!(write.value, DexValue::Int(expected_value));
        }
    }

    #[test]
    fn allocates_filled_new_array_range_as_pending_result() {
        let mut code = code_item(4, vec![0x0225, 0x1234, 0x0002, 0x000c, 0x0011]);
        code.ins_size = 2;
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::String(std::rc::Rc::from("left")),
                    DexValue::String(std::rc::Rc::from("right")),
                ]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Value(test_object(4660, "filled-array#4660"))
        );

        let array = handler
            .filled_new_array
            .expect("filled array should be recorded");
        assert_eq!(array.type_index, 0x1234);
        assert_eq!(array.first_register, 2);
        assert_eq!(array.argument_registers, vec![2, 3]);
        assert_eq!(
            array.values,
            vec![
                DexValue::String(std::rc::Rc::from("left")),
                DexValue::String(std::rc::Rc::from("right"))
            ]
        );
    }

    #[test]
    fn allocates_filled_new_array_as_pending_result() {
        let mut code = code_item(4, vec![0x2024, 0x1234, 0x0032, 0x000c, 0x0011]);
        code.ins_size = 2;
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::String(std::rc::Rc::from("left")),
                    DexValue::String(std::rc::Rc::from("right")),
                ]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Value(test_object(4660, "filled-array#4660"))
        );

        let array = handler
            .filled_new_array
            .expect("filled array should be recorded");
        assert_eq!(array.type_index, 0x1234);
        assert_eq!(array.first_register, 2);
        assert_eq!(array.argument_registers, vec![2, 3]);
        assert_eq!(
            array.values,
            vec![
                DexValue::String(std::rc::Rc::from("left")),
                DexValue::String(std::rc::Rc::from("right"))
            ]
        );
    }

    #[test]
    fn loads_const_class() {
        let code = code_item(1, vec![0x001c, 0x1234, 0x0011]);
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from("class#4660")))
        );

        let access = handler
            .class_access
            .expect("class access should be recorded");
        assert_eq!(access.type_index, 0x1234);
        assert_eq!(access.destination_register, 0);
    }

    #[test]
    fn decodes_check_cast() {
        let mut code = code_item(1, vec![0x001f, 0x1234, 0x0011]);
        code.ins_size = 1;
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[test_object(1, "example")]),
                &mut handler,
            )
            .expect("code should run"),
            DexExecutionResult::Value(test_object(1, "example"))
        );

        let check = handler.type_check.expect("type check should be recorded");
        assert_eq!(check.type_index, 0x1234);
        assert_eq!(check.register, 0);
    }

    #[test]
    fn executes_instance_of() {
        let mut code = code_item(2, vec![0x1020, 0x1234, 0x000f]);
        code.ins_size = 1;
        let mut handler = recording_handler();
        let object = test_object(1, "Lexample;");

        assert_eq!(
            interpret_code_with_initial_arguments(&code, Some(&[object.clone()]), &mut handler)
                .expect("code should run"),
            DexExecutionResult::Int(1)
        );

        let check = handler.instance_of.expect("instance-of should be recorded");
        assert_eq!(check.type_index, 0x1234);
        assert_eq!(check.destination_register, 0);
        assert_eq!(check.value_register, 1);
        assert_eq!(check.value, object);
    }

    #[test]
    fn executes_monitor_enter_and_exit_as_validated_noops() {
        let mut code = code_item(1, vec![0x001d, 0x001e, 0x000e]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Reference(std::rc::Rc::from("lock"))]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Void
        );
    }

    #[test]
    fn returns_throw_result_when_no_exception_handler_matches() {
        let mut code = code_item(1, vec![0x0027]);
        code.ins_size = 1;
        let thrown = test_object(7, "Ljava/lang/RuntimeException;");

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[thrown.clone()]),
                &mut recording_handler(),
            )
            .expect("throw should be represented as an execution result"),
            DexExecutionResult::Throw(thrown)
        );
    }

    #[test]
    fn dispatches_throw_to_matching_exception_handler() {
        let mut code = code_item(1, vec![0x0027, 0x000d, 0x1012, 0x000f]);
        code.ins_size = 1;
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 1,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 1,
            }],
        }];
        let thrown = test_object(7, "Ljava/lang/RuntimeException;");

        assert_eq!(
            interpret_code_with_initial_arguments(&code, Some(&[thrown]), &mut recording_handler(),)
                .expect("throw should be caught"),
            DexExecutionResult::Int(1)
        );
    }

    // cite: art interpreter_switch_impl-inl.h#THROW, MOVE_EXCEPTION
    // cite: art interpreter_common.cc#MoveToExceptionHandler
    #[test]
    fn art_throw_null_dispatches_generated_null_pointer_exception_to_typed_handler() {
        let mut code = code_item(1, vec![0x0027, 0x000d, 0x0011]);
        code.ins_size = 1;
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 1,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/NullPointerException;".to_owned()),
                address: 1,
            }],
        }];
        let expected = test_object(0x4e50, "Ljava/lang/NullPointerException;");
        let mut handler = recording_handler();

        assert_eq!(
            interpret_code_with_initial_arguments(&code, Some(&[DexValue::Null]), &mut handler,)
                .expect("throw null should enter the typed handler"),
            DexExecutionResult::Value(expected)
        );
        assert_eq!(
            handler.generated_exception,
            Some(DexGeneratedException {
                class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
                detail_message: None,
            })
        );
    }

    // cite: art interpreter_switch_impl-inl.h#THROW, MOVE_EXCEPTION
    // cite: art art_method.cc#FindCatchBlock
    #[test]
    fn art_throw_integer_zero_reaches_catch_all_after_nonmatching_typed_handler() {
        let mut code = code_item(1, vec![0x0012, 0x0027, 0x000d, 0x0011]);
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 1,
            instruction_count: 1,
            handlers: vec![
                crate::dex::DexExceptionHandler {
                    type_descriptor: Some("Ljava/lang/IllegalArgumentException;".to_owned()),
                    address: 0,
                },
                crate::dex::DexExceptionHandler {
                    type_descriptor: None,
                    address: 2,
                },
            ],
        }];
        let expected = test_object(0x4e50, "Ljava/lang/NullPointerException;");

        assert_eq!(
            interpret_code_with_handler(&code, &mut recording_handler())
                .expect("encoded null should enter the catch-all handler"),
            DexExecutionResult::Value(expected)
        );
    }

    // cite: art interpreter_switch_impl-inl.h#THROW, HandlePendingException
    // cite: art art_method.cc#FindCatchBlock
    #[test]
    fn art_throw_null_propagates_generated_exception_when_typed_handler_does_not_match() {
        let mut code = code_item(1, vec![0x0027]);
        code.ins_size = 1;
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 1,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/IllegalArgumentException;".to_owned()),
                address: 0,
            }],
        }];
        let expected = test_object(0x4e50, "Ljava/lang/NullPointerException;");

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Null]),
                &mut recording_handler(),
            )
            .expect("uncaught throw null should remain a guest exception"),
            DexExecutionResult::Throw(expected)
        );
    }

    // cite: art interpreter_switch_impl-inl.h#PossiblyHandlePendingExceptionOnInvoke, MOVE_EXCEPTION
    // cite: art interpreter_common.cc#MoveToExceptionHandler
    #[test]
    fn art_invocation_throw_uses_the_same_exception_dispatch_path() {
        let mut code = code_item(1, vec![0x0071, 0x0001, 0x0000, 0x000d, 0x0011]);
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 3,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 3,
            }],
        }];
        let thrown = test_object(7, "Ljava/lang/RuntimeException;");
        let mut handler = recording_handler();
        handler.invocation_result = DexInvocationResult::Throw(thrown.clone());

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler)
                .expect("invocation exception should enter the typed handler"),
            DexExecutionResult::Value(thrown)
        );
    }

    // cite: art interpreter_common.cc#MoveToExceptionHandler
    // cite: art art_method.cc#FindCatchBlock
    #[test]
    fn art_handler_without_move_exception_clears_pending_exception_before_entry() {
        let mut code = code_item(0, vec![0x000e, 0x000e]);
        code.tries_size = 1;
        code.try_blocks = vec![crate::dex::DexTryBlock {
            start_address: 0,
            instruction_count: 1,
            handlers: vec![crate::dex::DexExceptionHandler {
                type_descriptor: Some("Ljava/lang/RuntimeException;".to_owned()),
                address: 1,
            }],
        }];
        let thrown = test_object(7, "Ljava/lang/RuntimeException;");
        let mut pending_result = Some(DexPendingResult::from_value(DexValue::Int(1)));
        let mut pending_exception = Some(test_object(8, "Ljava/lang/RuntimeException;"));
        let mut pc = 0;

        assert_eq!(
            dispatch_exception(
                &code,
                0,
                thrown,
                &mut pending_result,
                &mut pending_exception,
                &mut pc,
                &mut recording_handler(),
            )
            .expect("handler dispatch should succeed"),
            None
        );
        assert!(pending_result.is_none());
        assert!(pending_exception.is_none());
        assert_eq!(pc, 1);
    }

    #[test]
    fn executes_if_nez_branch() {
        let code = code_item(1, vec![0x1012, 0x0039, 0x0003, 0x2012, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_if_eqz_branch_for_null() {
        let mut code = code_item(1, vec![0x0038, 0x0003, 0x0004, 0x0011]);
        code.ins_size = 1;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[DexValue::Null]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Null)
        );
    }

    #[test]
    fn executes_goto_branch() {
        let code = code_item(0, vec![0x0228, 0x0004, 0x000e]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Void
        );
    }

    #[test]
    fn executes_goto16_branch() {
        let code = code_item(0, vec![0x0029, 0x0003, 0x0004, 0x000e]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Void
        );
    }

    #[test]
    fn executes_goto32_branch() {
        let code = code_item(0, vec![0x002a, 0x0004, 0x0000, 0x0004, 0x000e]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Void
        );
    }

    #[test]
    fn executes_packed_switch_branch() {
        let code = code_item(
            1,
            vec![
                0x1012, 0x002b, 0x0005, 0x0000, 0x0012, 0x000f, 0x0100, 0x0002, 0x0001, 0x0000,
                0x000d, 0x0000, 0x000f, 0x0000, 0x2012, 0x000f, 0x3012, 0x000f,
            ],
        );

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(2)
        );
    }

    #[test]
    fn executes_sparse_switch_branch() {
        let code = code_item(
            1,
            vec![
                0x0013, 0x0009, 0x002c, 0x0008, 0x0000, 0x0012, 0x000f, 0x4012, 0x000f, 0x000f,
                0x0200, 0x0002, 0x0004, 0x0000, 0x0009, 0x0000, 0x0005, 0x0000, 0x0007, 0x0000,
            ],
        );

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(9)
        );
    }

    #[test]
    fn executes_if_ltz_branch() {
        let code = code_item(1, vec![0xf012, 0x003a, 0x0003, 0x2012, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(-1)
        );
    }

    #[test]
    fn executes_if_lt_branch() {
        let code = code_item(2, vec![0x1012, 0x2112, 0x1034, 0x0003, 0x0012, 0x000f]);

        assert_eq!(
            interpret_without_invocations(&code).expect("code should run"),
            DexExecutionResult::Int(1)
        );
    }

    #[test]
    fn executes_if_eq_branch_for_matching_references() {
        let mut code = code_item(3, vec![0x2132, 0x0003, 0x0004, 0x0111]);
        code.ins_size = 2;

        assert_eq!(
            interpret_code_with_initial_arguments(
                &code,
                Some(&[
                    DexValue::Reference(std::rc::Rc::from("same")),
                    DexValue::Reference(std::rc::Rc::from("same")),
                ]),
                &mut recording_handler(),
            )
            .expect("code should run"),
            DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from("same")))
        );
    }

    #[test]
    fn moves_object_invocation_result() {
        let code = code_item(1, vec![0x0071, 0x0001, 0x0000, 0x000c, 0x0011]);
        let mut handler = ReturningInvocationHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from("result")))
        );
    }

    #[test]
    fn moves_wide_invocation_result() {
        let code = code_item(2, vec![0x0071, 0x0001, 0x0000, 0x000b, 0x0011]);
        let mut handler = ReturningLongInvocationHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(123))
        );
    }

    #[test]
    fn art_interpreter_moves_raw_int_invocation_result() {
        let code = code_item(1, vec![0x0071, 0x0001, 0x0000, 0x000a, 0x000f]);
        let mut handler = ReturningRawIntInvocationHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Int(77)
        );
    }

    #[test]
    fn art_interpreter_moves_raw_wide_invocation_result() {
        let code = code_item(2, vec![0x0071, 0x0001, 0x0000, 0x000b, 0x0010]);
        let mut handler = ReturningRawLongInvocationHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(456))
        );
    }

    #[test]
    fn move_result_wide_initializes_high_register_slot() {
        let code = code_item(2, vec![0x0071, 0x0001, 0x0000, 0x000b, 0x0111]);
        let mut handler = ReturningLongInvocationHandler;

        assert_eq!(
            interpret_code_with_handler(&code, &mut handler).expect("code should run"),
            DexExecutionResult::Value(DexValue::Long(123))
        );
    }

    // cite: art reflection.cc#GetCallingClass
    #[test]
    fn parked_invocation_resumes_after_call_site() {
        let code = code_item(1, vec![0x0071, 0x0001, 0x0000, 0x7012, 0x000f]);
        let mut handler = ParkingOnceInvocationHandler { invocations: 0 };

        let parked = interpret_code_with_handler(&code, &mut handler).expect("code should park");
        let DexExecutionResult::Parked(mut execution) = parked else {
            panic!("expected parked execution, got {parked:?}");
        };
        let Some(DexParkedStep::Frame(frame)) = execution.pop_next_step() else {
            panic!("parked execution should contain one frame");
        };
        assert!(execution.pop_next_step().is_none());
        assert_eq!(
            frame.current_class_descriptor(),
            Some("Ltest/ParkedCaller;")
        );

        let result = resume_parked_frame(frame, DexInvocationResult::Void, &mut handler)
            .expect("parked frame should resume");

        assert_eq!(result, DexExecutionResult::Int(7));
        assert_eq!(handler.invocations, 1);
    }

    struct ReturningInvocationHandler;

    impl DexInvocationHandler for ReturningInvocationHandler {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Ok(DexInvocationResult::Value(DexValue::Reference(
                std::rc::Rc::from("result"),
            )))
        }
    }

    struct ReturningLongInvocationHandler;

    impl DexInvocationHandler for ReturningLongInvocationHandler {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Ok(DexInvocationResult::Value(DexValue::Long(123)))
        }
    }

    struct ReturningRawIntInvocationHandler;

    impl DexInvocationHandler for ReturningRawIntInvocationHandler {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Ok(DexInvocationResult::RawValue(DexPendingResult::from_value(
                DexValue::Int(77),
            )))
        }
    }

    struct ReturningRawLongInvocationHandler;

    impl DexInvocationHandler for ReturningRawLongInvocationHandler {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Ok(DexInvocationResult::RawValue(DexPendingResult::from_value(
                DexValue::Long(456),
            )))
        }
    }

    struct ParkingOnceInvocationHandler {
        invocations: usize,
    }

    impl DexInvocationHandler for ParkingOnceInvocationHandler {
        fn current_class_descriptor(&self) -> Option<&str> {
            Some("Ltest/ParkedCaller;")
        }

        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            self.invocations += 1;
            Ok(DexInvocationResult::Parked(DexParkedExecution::empty()))
        }
    }

    struct LongFieldReadHandler;

    impl DexInvocationHandler for LongFieldReadHandler {
        fn invoke(&mut self, _invocation: DexInvocation) -> Result<DexInvocationResult> {
            Err(WinedroidError::new(
                "LongFieldReadHandler does not support invocation",
            ))
        }

        fn read_instance_field(&mut self, _access: DexFieldAccess) -> Result<DexValue> {
            Ok(DexValue::Long(55))
        }

        fn read_static_field(
            &mut self,
            _access: DexStaticFieldAccess,
        ) -> Result<DexStaticFieldReadResult> {
            Ok(DexStaticFieldReadResult::Value(DexValue::Long(55)))
        }
    }

    struct RecordingInvocationHandler {
        invocation: Option<DexInvocation>,
        invocation_result: DexInvocationResult,
        field_access: Option<DexFieldAccess>,
        field_write: Option<DexFieldWrite>,
        static_field_access: Option<DexStaticFieldAccess>,
        static_field_write: Option<DexStaticFieldWrite>,
        string_access: Option<DexStringAccess>,
        class_access: Option<DexClassAccess>,
        new_instance: Option<DexNewInstance>,
        new_array: Option<DexNewArray>,
        array_length: Option<DexArrayLength>,
        array_read: Option<DexArrayRead>,
        array_read_value: DexValue,
        array_write: Option<DexArrayWrite>,
        array_writes: Vec<DexArrayWrite>,
        filled_new_array: Option<DexFilledNewArray>,
        type_check: Option<DexTypeCheck>,
        check_cast_result: DexCheckCastResult,
        instance_of: Option<DexInstanceOf>,
        generated_exception: Option<DexGeneratedException>,
    }

    impl DexInvocationHandler for RecordingInvocationHandler {
        fn invoke(&mut self, invocation: DexInvocation) -> Result<DexInvocationResult> {
            self.invocation = Some(invocation);
            Ok(self.invocation_result.clone())
        }

        fn allocate_generated_exception(
            &mut self,
            exception: DexGeneratedException,
        ) -> Result<DexValue> {
            let class_descriptor = exception.class_descriptor.clone();
            self.generated_exception = Some(exception);
            Ok(test_object(0x4e50, &class_descriptor))
        }

        fn read_instance_field(&mut self, access: DexFieldAccess) -> Result<DexValue> {
            self.field_access = Some(access);
            Ok(DexValue::Int(0))
        }

        fn write_instance_field(&mut self, write: DexFieldWrite) -> Result<()> {
            self.field_write = Some(write);
            Ok(())
        }

        fn read_static_field(
            &mut self,
            access: DexStaticFieldAccess,
        ) -> Result<DexStaticFieldReadResult> {
            self.static_field_access = Some(access);
            Ok(DexStaticFieldReadResult::Value(DexValue::Int(0)))
        }

        fn write_static_field(
            &mut self,
            write: DexStaticFieldWrite,
        ) -> Result<DexStaticFieldWriteResult> {
            self.static_field_write = Some(write);
            Ok(DexStaticFieldWriteResult::Complete)
        }

        fn load_string(&mut self, access: DexStringAccess) -> Result<DexValue> {
            let value = format!("string#{}", access.string_index);
            self.string_access = Some(access);
            Ok(DexValue::String(std::rc::Rc::from(value)))
        }

        fn load_class(&mut self, access: DexClassAccess) -> Result<DexValue> {
            let value = format!("class#{}", access.type_index);
            self.class_access = Some(access);
            Ok(DexValue::Reference(std::rc::Rc::from(value)))
        }

        fn allocate_object(&mut self, instance: DexNewInstance) -> Result<DexNewInstanceResult> {
            let value = format!("type#{}", instance.type_index);
            let id = u64::from(instance.type_index);
            self.new_instance = Some(instance);
            Ok(DexNewInstanceResult::Allocated(test_object(id, &value)))
        }

        fn allocate_array(&mut self, array: DexNewArray) -> Result<DexValue> {
            let value = format!("array#{}", array.type_index);
            let id = u64::from(array.type_index);
            self.new_array = Some(array);
            Ok(test_object(id, &value))
        }

        fn array_length(&mut self, length: DexArrayLength) -> Result<i32> {
            self.array_length = Some(length);
            Ok(3)
        }

        fn read_array(&mut self, read: DexArrayRead) -> Result<DexValue> {
            self.array_read = Some(read);
            Ok(self.array_read_value.clone())
        }

        fn write_array(&mut self, write: DexArrayWrite) -> Result<()> {
            self.array_write = Some(write.clone());
            self.array_writes.push(write);
            Ok(())
        }

        fn allocate_filled_array(&mut self, array: DexFilledNewArray) -> Result<DexValue> {
            let value = format!("filled-array#{}", array.type_index);
            let id = u64::from(array.type_index);
            self.filled_new_array = Some(array);
            Ok(test_object(id, &value))
        }

        fn check_cast(&mut self, check: DexTypeCheck) -> Result<DexCheckCastResult> {
            self.type_check = Some(check);
            Ok(self.check_cast_result.clone())
        }

        fn instance_of(&mut self, check: DexInstanceOf) -> Result<i32> {
            self.instance_of = Some(check);
            Ok(1)
        }
    }

    fn interpret_without_invocations(code: &DexCodeItem) -> Result<DexExecutionResult> {
        let mut handler = recording_handler();
        interpret_code_with_handler(code, &mut handler)
    }

    fn recording_handler() -> RecordingInvocationHandler {
        RecordingInvocationHandler {
            invocation: None,
            invocation_result: DexInvocationResult::Void,
            field_access: None,
            field_write: None,
            static_field_access: None,
            static_field_write: None,
            string_access: None,
            class_access: None,
            new_instance: None,
            new_array: None,
            array_length: None,
            array_read: None,
            array_read_value: DexValue::String(std::rc::Rc::from("array-value")),
            array_write: None,
            array_writes: Vec::new(),
            filled_new_array: None,
            type_check: None,
            check_cast_result: DexCheckCastResult::Success,
            instance_of: None,
            generated_exception: None,
        }
    }

    fn test_object(id: u64, class_descriptor: &str) -> DexValue {
        DexValue::Object(DexObjectRef {
            id,
            class_descriptor: std::rc::Rc::from(class_descriptor.to_owned()),
        })
    }

    fn code_item(registers_size: u16, instructions: Vec<u16>) -> DexCodeItem {
        DexCodeItem {
            registers_size,
            ins_size: 0,
            outs_size: 0,
            tries_size: 0,
            debug_info_offset: 0,
            instructions,
            try_blocks: Vec::new(),
        }
    }

    #[cfg(all(feature = "jit", not(target_family = "wasm")))]
    #[test]
    fn measure_jit_vs_interpreter_on_an_integer_loop() {
        use std::time::Instant;

        let bytecode: Vec<u16> = vec![
            0x003d, 0x0007, 0x0190, 0x0001, 0x0091, 0x0200, 0xfa28, 0x010f,
        ];
        let n: i32 = 4000;
        let expected = DexExecutionResult::Int(n * (n + 1) / 2);

        let mut code = code_item(3, bytecode.clone());
        code.ins_size = 3;
        let args = [DexValue::Int(n), DexValue::Int(0), DexValue::Int(1)];
        let mut handler = recording_handler();
        let iterations = 200;
        let interp_start = Instant::now();
        for _ in 0..iterations {
            let result = interpret_code_with_initial_arguments_in_context(
                &code,
                Some(&args),
                &mut handler,
                0,
            )
            .expect("interpret");
            assert_eq!(result, expected);
        }
        let interp = interp_start.elapsed();

        let ops = crate::jit::decode_integer_subset(&bytecode).expect("decode");
        let compiled = crate::jit::compile(&ops, 3).expect("compile");
        let registers = [n, 0, 1];
        assert_eq!(compiled.call(&registers), n * (n + 1) / 2);
        let jit_start = Instant::now();
        for _ in 0..iterations {
            assert_eq!(compiled.call(&registers), n * (n + 1) / 2);
        }
        let jit = jit_start.elapsed();

        let ratio = interp.as_secs_f64() / jit.as_secs_f64().max(1e-9);
        eprintln!(
            "[jit-measure] loop n={n} x{iterations}: interpreter={:?} jit={:?} speedup={ratio:.1}x",
            interp, jit
        );
    }
}
