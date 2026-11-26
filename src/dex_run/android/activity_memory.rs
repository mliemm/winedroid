use super::*;
use crate::native::bionic::host_stats;

const MEMORY_INFO: &str = "Landroid/app/ActivityManager$MemoryInfo;";
const KIB: i64 = 1024;
const MIB: i64 = KIB * KIB;

impl DexProgram {
    pub(in crate::dex_run) fn activity_manager_memory_info(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let [receiver, info] = invocation.arguments.as_slice() else {
            return None;
        };
        if !self.value_matches_type(receiver, "Landroid/app/ActivityManager;") {
            return None;
        }
        // cite: frameworks-base ActivityManager.java#getMemoryInfo, readFromParcel
        // cite: frameworks-base IActivityManager.aidl#getMemoryInfo
        if matches!(info, DexValue::Null | DexValue::Int(0)) {
            return Some(DexExecutionResult::Throw(
                self.allocate_object("Ljava/lang/NullPointerException;".to_owned()),
            ));
        }
        if !self.value_matches_type(info, MEMORY_INFO) {
            return None;
        }

        let memory = host_stats::memory();
        let total = i64::try_from(memory.total)
            .ok()
            .filter(|total| *total > 0)?;
        // cite: frameworks-base android_util_Process.cpp#android_os_Process_getFreeMemory
        if !memory.vm_stats_available {
            return Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/RuntimeException;".to_owned(),
                    detail_message: Some("Could not read system memory statistics".to_owned()),
                }),
            ));
        }
        let available = i64::try_from(memory.free / 1024 + memory.cached / 1024)
            .ok()?
            .checked_mul(KIB)?;
        let fields = memory_info_fields(total, available, self.display_size());
        for (name, value) in fields {
            let field = DexFieldId {
                class_descriptor: MEMORY_INFO.to_owned(),
                name: name.to_owned(),
                type_descriptor: match value {
                    DexValue::Int(_) => "Z",
                    _ => "J",
                }
                .to_owned(),
            };
            self.write_stored_instance_field(info, &field, value);
        }
        Some(DexExecutionResult::Void)
    }
}

fn memory_info_fields(
    total: i64,
    available: i64,
    display: DisplaySize,
) -> [(&'static str, DexValue); 9] {
    // cite: frameworks-base ProcessList.java#getMemoryInfo, updateOomLevels, getMemLevel, mOomMinFreeLow, mOomMinFreeHigh
    // cite: frameworks-base config.xml#config_lowMemoryKillerMinFreeKbytesAbsolute, config_lowMemoryKillerMinFreeKbytesAdjust
    let memory_scale = (total / MIB - 350) as f32 / 350.0;
    let pixels = (display.width() * display.height()) as f32;
    let display_scale = (pixels - 480.0 * 800.0) / ((1280.0 - 480.0) * 800.0);
    let scale = memory_scale.max(display_scale).clamp(0.0, 1.0);
    let level = |low: i32, high: i32| (low as f32 + (high - low) as f32 * scale) as i64 * KIB;
    let foreground = level(12288, 73728);
    let visible = level(18432, 92160);
    let cached = level(43008, 147456 * 3 / 2);
    // cite: frameworks-base ProcessList.java#HOME_APP_ADJ, SERVICE_ADJ, CACHED_APP_MIN_ADJ, mOomAdj
    let home = cached;
    let service = cached;

    [
        ("advertisedMem", DexValue::Long(advertised_memory(total))),
        ("availMem", DexValue::Long(available)),
        ("totalMem", DexValue::Long(total)),
        ("threshold", DexValue::Long(home)),
        (
            "lowMemory",
            DexValue::Int(i32::from(available < home + (cached - home) / 2)),
        ),
        ("hiddenAppThreshold", DexValue::Long(cached)),
        ("secondaryServerThreshold", DexValue::Long(service)),
        ("visibleAppThreshold", DexValue::Long(visible)),
        ("foregroundAppThreshold", DexValue::Long(foreground)),
    ]
}

fn advertised_memory(total: i64) -> i64 {
    // cite: frameworks-base Process.java#getAdvertisedMem
    // cite: frameworks-base FileUtils.java#roundStorageSize
    let mut value = 1u64;
    let mut binary_unit = 1;
    let mut decimal_unit = 1;
    while value * binary_unit < total as u64 {
        value *= 2;
        if value > 512 {
            value = 1;
            binary_unit *= 1024;
            decimal_unit *= 1000;
        }
    }
    (value * decimal_unit) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: frameworks-base ProcessList.java#getMemoryInfo, updateOomLevels
    // cite: frameworks-base FileUtils.java#roundStorageSize
    #[test]
    fn memory_info_uses_device_thresholds_and_free_memory() {
        let small = DisplaySize::new(420, 760).unwrap();
        let half = DisplaySize::new(880, 800).unwrap();
        let cases = [
            (350 * MIB, small, 43008 * KIB, 512_000_000),
            (350 * MIB, half, 132096 * KIB, 512_000_000),
            (525 * MIB, small, 132096 * KIB, 1_000_000_000),
            (8 * 1024 * MIB, small, 221184 * KIB, 8_000_000_000),
        ];
        for (total, display, threshold, advertised) in cases {
            for available in [threshold - 1, threshold] {
                let fields =
                    std::collections::HashMap::from(memory_info_fields(total, available, display));
                assert_eq!(fields["threshold"], DexValue::Long(threshold));
                assert_eq!(fields["availMem"], DexValue::Long(available));
                assert_eq!(fields["totalMem"], DexValue::Long(total));
                assert_eq!(fields["advertisedMem"], DexValue::Long(advertised));
                assert_eq!(
                    fields["lowMemory"],
                    DexValue::Int(i32::from(available < threshold))
                );
            }
        }
    }
}
