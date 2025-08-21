use alvr_common::RelaxedAtomic;
use alvr_session::CodecType;

const NAL_PREFIX_3B: [u8; 3] = [0x00, 0x00, 0x01];
const NAL_PREFIX_4B: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

const H264_NAL_TYPE_SPS: u8 = 7;
const HEVC_NAL_TYPE_VPS: u8 = 32;

const H264_NAL_TYPE_AUD: u8 = 9;
const HEVC_NAL_TYPE_AUD: u8 = 35;

// Returns the size of the prefix
fn nal_prefix_size(buf: &[u8]) -> Option<usize> {
    if buf.starts_with(&NAL_PREFIX_3B) {
        Some(NAL_PREFIX_3B.len())
    } else if buf.starts_with(&NAL_PREFIX_4B) {
        Some(NAL_PREFIX_4B.len())
    } else {
        None
    }
}

// Obtain the (VPS +) SPS + PPS video configuration headers from H.264 or H.265 stream as a sequence
// of NALs. (VPS +) SPS + PPS have short size (8 bytes + 28 bytes in some environment), so we can
// assume SPS + PPS is contained in first fragment.
fn obtain_headers<'a>(buf: &mut &'a [u8], nal_count: usize) -> Option<&'a [u8]> {
    let mut cursor = 0;
    let mut found_headers_count = 0;

    while cursor <= buf.len() {
        if cursor + NAL_PREFIX_4B.len() > buf.len() {
            cursor += 1;
            continue;
        } else if let Some(prefix_size) = nal_prefix_size(&buf[cursor..]) {
            found_headers_count += 1;

            // We want the cursor to point to the first non-header nal, hence the + 1
            if found_headers_count == nal_count as isize + 1 {
                break;
            }

            cursor += prefix_size;
        } else {
            cursor += 1;
            continue;
        }
    }

    if found_headers_count != nal_count as isize + 1 {
        return None;
    }

    let (headers, buffer) = buf.split_at(cursor);
    *buf = buffer;

    Some(headers)
}

fn process_h264_nals<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
    let mut prefix_size = nal_prefix_size(buf)?;
    let mut nal_type = buf[prefix_size] & 0x1F;

    if nal_type == H264_NAL_TYPE_AUD && buf.len() > prefix_size * 2 + 2 {
        *buf = &buf[prefix_size + 2..];

        // Cannot fail because of if condition
        prefix_size = nal_prefix_size(buf)?;
        nal_type = buf[prefix_size] & 0x1F;
    }

    if nal_type == H264_NAL_TYPE_SPS {
        obtain_headers(buf, 2) // 2 headers SPS and PPS
    } else {
        None
    }
}

fn process_hevc_nals<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
    let mut prefix_size = nal_prefix_size(buf)?;
    let mut nal_type = (buf[prefix_size] >> 1) & 0x3F;

    if nal_type == HEVC_NAL_TYPE_AUD && buf.len() > prefix_size * 2 + 3 {
        *buf = &buf[prefix_size + 3..];

        // Cannot fail because of if condition
        prefix_size = nal_prefix_size(buf)?;
        nal_type = (buf[prefix_size] >> 1) & 0x3F;
    }

    if nal_type == HEVC_NAL_TYPE_VPS {
        obtain_headers(buf, 3) // 3 headers VPS, SPS and PPS
    } else {
        None
    }
}

/// Remove config NALs from the buffer and return them
/// Returns None if buffer is too short
/// Returns Some(None) if no config NALs are present
/// Returns Some(Some(...)) if config NALs are present
pub fn parse_nals<'a>(codec: CodecType, buffer: &mut &'a [u8]) -> Option<Option<&'a [u8]>> {
    static AV1_GOT_FRAME: RelaxedAtomic = RelaxedAtomic::new(false);

    if buffer.len() < NAL_PREFIX_4B.len() {
        return None;
    }

    Some(match codec {
        CodecType::H264 => process_h264_nals(buffer),
        CodecType::Hevc => process_hevc_nals(buffer),
        CodecType::AV1 if !AV1_GOT_FRAME.value() => {
            AV1_GOT_FRAME.set(true);
            Some(&[])
        }
        _ => None,
    })
}
