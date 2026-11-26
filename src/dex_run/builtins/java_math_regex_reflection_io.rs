use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_math_regex_reflection_io_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if let Some(result) = self.invoke_java_math_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_regex_util_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_reflection_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = invocation.and_then(|invocation| {
            self.invoke_java_zip_method(method, invocation, 0)
                .ok()
                .flatten()
        }) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_io_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_nio_builtin(method, invocation) {
            return Some(result);
        }

        None
    }
}
