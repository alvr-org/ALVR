# ExpectWirelessHeadset Race Condition Fix - Implementation Guide

## Issue #3320: SteamVR 2.16.7 fails to initialize with ALVR

### Problem Summary
When connecting ALVR to SteamVR 2.16.7+, the connection immediately fails because SteamVR reads the `ExpectWirelessHeadset` property BEFORE ALVR's driver has finished initializing and setting the property value.

### Root Cause
Race condition between:
1. **SteamVR**: Activates tracked device (HMD), then immediately queries `ExpectWirelessHeadset` (~2.6ms later)
2. **ALVR Driver**: Has not yet set the `ExpectWirelessHeadset` property to `true`
3. **Result**: SteamVR reads property value as `false` (default/uninitialized)
4. **Consequence**: SteamVR treats wireless headset as wired, triggers restart sequence → fails

### SteamVR Error Messages
```
[Info] - Tracked device activated: 0
[Info] - CheckHmdDriverName: ActualTrackingSystemName: alvr_server (0)
[Info] - ExpectWirelessHeadset: 0  ← Wrong! Should be 1
[Info] - [System] Transition from 'SteamVRSystemState_Ready' to 'SteamVRSystemState_Restart'
...
[Info] - [System] Transition from 'SteamVRSystemState_Restart' to 'SteamVRSystemState_Shutdown'
```

## Solution: Synchronous Property Initialization

### Key Principle
**ALL critical OpenVR properties must be set BEFORE the device is reported as activated to SteamVR.**

### Implementation Strategy

#### File: `alvr/server_openvr/src/props.rs`

Add synchronous property initialization function that sets all required properties before device activation:

```rust
/// Critical OpenVR properties that must be set BEFORE device activation
const CRITICAL_PROPERTIES: &[&str] = &[
    "ExpectWirelessHeadset",
    "DeviceIsWirelessBool",
    "ConnectedWirelessDongleString",
];

/// Initialize all critical properties for HMD device (ID 0)
/// This MUST be called synchronously before TrackedDeviceActivated is reported
pub fn initialize_critical_hmd_properties(
    instance_ptr: Option<*mut c_void>,
    is_wireless: bool,
) -> Result<()> {
    if instance_ptr.is_none() {
        return Ok(()); // Driver not yet ready
    }

    // Set ExpectWirelessHeadset based on headset type
    let wireless_prop = OpenvrProperty {
        key: OpenvrPropKey::ExpectWirelessHeadset,
        value: if is_wireless { "1" } else { "0" }.to_string(),
    };
    set_openvr_prop(instance_ptr, *HEAD_ID, wireless_prop);

    // Set DeviceIsWirelessBool (redundant but explicit)
    let device_wireless_prop = OpenvrProperty {
        key: OpenvrPropKey::DeviceIsWirelessBool,
        value: if is_wireless { "true" } else { "false" }.to_string(),
    };
    set_openvr_prop(instance_ptr, *HEAD_ID, device_wireless_prop);

    // Set wireless dongle info if wireless
    if is_wireless {
        let dongle_prop = OpenvrProperty {
            key: OpenvrPropKey::ConnectedWirelessDongleString,
            value: "D0000BE000".to_string(),
        };
        set_openvr_prop(instance_ptr, *HEAD_ID, dongle_prop);
    }

    Ok(())
}
```

#### File: `alvr/server_core/src/connection.rs`

Call property initialization immediately after creating the device, before device activation:

```rust
// After ServerCoreContext is created, BEFORE any device activation
if let Some(instance_ptr) = get_openvr_instance_ptr() {
    let is_wireless = matches!(
        &initial_settings.headset.emulation_mode,
        HeadsetEmulationMode::Quest1 | HeadsetEmulationMode::Quest2
    );
    
    // Initialize all critical properties BEFORE device activation
    if let Err(e) = alvr_server_openvr::initialize_critical_hmd_properties(
        instance_ptr,
        is_wireless,
    ) {
        warn!("Failed to initialize critical properties: {e:?}");
    }
}
```

#### File: `alvr/session/src/settings.rs`

Add `OpenvrPropKey::ExpectWirelessHeadset` enum variant if not already present:

```rust
pub enum OpenvrPropKey {
    // ... existing properties ...
    ExpectWirelessHeadset,
    DeviceIsWirelessBool,
    ConnectedWirelessDongleString,
}
```

### Timing Diagram

```
BEFORE FIX:
────────────────────────────────────────────────
ALVR:     [Create Device] ........[Set Props]
SteamVR:                [Activate Device][Query Props]
Result:                               ✗ Read false

AFTER FIX:
────────────────────────────────────────────────
ALVR:     [Create Device][Set Props][Signal Ready]
SteamVR:                              [Activate Device][Query Props]
Result:                                             ✓ Read true
```

## Implementation Checklist

### Phase 1: Add Property Management
- [ ] Add `initialize_critical_hmd_properties()` function to `props.rs`
- [ ] Add required `OpenvrPropKey` enum variants to `settings.rs`
- [ ] Add property key mappings in `props.rs`

### Phase 2: Integrate into Connection Flow
- [ ] Identify device creation point in `server_core/connection.rs`
- [ ] Call `initialize_critical_hmd_properties()` immediately after device creation
- [ ] Ensure call is SYNCHRONOUS (not in separate thread)
- [ ] Add debug logging to verify properties are set

### Phase 3: Testing
- [ ] Connect Meta Quest 2 to SteamVR
- [ ] Verify `ExpectWirelessHeadset: 1` in vrmonitor.txt
- [ ] Verify no Restart→Shutdown transition
- [ ] Check vrserver.txt for no errors related to properties

## Testing Plan

### Test Case 1: Wireless Headset (Quest 2)
```
Setup:
  - Configure ALVR for Quest2 emulation mode
  - Connect Quest 2 via ALVR

Expected Results:
  - vrmonitor.txt shows "ExpectWirelessHeadset: 1"
  - No Restart→Shutdown sequence
  - Game loads successfully in headset

Verification:
  grep "ExpectWirelessHeadset" ~/.steam/steamapps/common/SteamVR/logs/vrmonitor.txt
  → Should show "ExpectWirelessHeadset: 1"
```

### Test Case 2: Wired Headset (Rift S Emulation)
```
Setup:
  - Configure ALVR for Rift S emulation mode
  - Connect via ALVR

Expected Results:
  - vrmonitor.txt shows "ExpectWirelessHeadset: 0"
  - No unusual transitions
  - Headset displays normally
```

## Rollout Strategy

1. **Verify Fix**: Test with both Quest and Rift S emulation modes
2. **Create PR**: Submit with detailed testing results
3. **Documentation**: Update wiki with property initialization requirements
4. **Release Notes**: Document fix in changelog

## Related OpenVR Properties

For reference, here are other important wireless-related properties:

| Property | Type | Wireless | Wired |
|----------|------|----------|-------|
| ExpectWirelessHeadset | bool | 1 | 0 |
| DeviceIsWirelessBool | bool | true | false |
| ConnectedWirelessDongleString | string | "D..." | empty |
| WirelessSeparateControllerAndDongleBool | bool | true | false |
| DeviceProvidesBatteryStatus | bool | true | false |

## Fallback Handling

If property initialization fails:
1. Log warning but don't abort connection
2. SteamVR will auto-detect wireless based on behavior
3. Connection may still work but with delay/workarounds

## Performance Impact

- **Minimal**: Property setting is synchronous, happens once at connection time
- **Latency**: <1ms overhead
- **No impact** on streaming performance

## Related Issues and PRs

- Issue #3320: SteamVR 2.16.7 fails to initialize with ALVR
- May fix issues: #3304 (SteamVR settings issues), #3308 (connection errors)

