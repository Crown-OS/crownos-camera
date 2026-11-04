//! What a camera stream can deliver, and the SPA pods that offer it.

use std::io::Cursor;

use pipewire::spa::buffer::DataType;
use pipewire::spa::param::ParamType;
use pipewire::spa::param::format::{FormatProperties, MediaSubtype, MediaType};
use pipewire::spa::param::video::VideoFormat;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{self, ChoiceValue, Pod, Property, Value};
use pipewire::spa::sys::SPA_PARAM_BUFFERS_dataType;
use pipewire::spa::utils::{Choice, ChoiceEnum, ChoiceFlags, Fraction, Rectangle, SpaTypes};

use super::CaptureError;

/// How a camera encodes its frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraFormat {
    Mjpeg,
    Yuyv,
    Nv12,
}

/// The size and rate to ask the camera for. It answers with the nearest it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraRequest {
    pub width: u32,
    pub height: u32,
    pub frames_per_second: u32,
}

impl Default for CameraRequest {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            frames_per_second: 30,
        }
    }
}

/// What the stream settled on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Negotiated {
    pub format: CameraFormat,
    pub width: u32,
    pub height: u32,
}

const MAX_SIZE: Rectangle = Rectangle {
    width: 8192,
    height: 8192,
};
const MIN_SIZE: Rectangle = Rectangle {
    width: 1,
    height: 1,
};

fn serialize(object: pod::Object) -> Result<Vec<u8>, CaptureError> {
    PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Object(object))
        .map(|(cursor, _)| cursor.into_inner())
        .map_err(|_| CaptureError::Negotiation)
}

/// Buffers in memory the CPU can read, which is where frames are decoded. A
/// camera left to choose may hand out dma-bufs, which a stream that never said
/// it takes them refuses.
pub fn cpu_buffers() -> Result<Vec<u8>, CaptureError> {
    let readable = (1 << DataType::MemPtr.as_raw()) | (1 << DataType::MemFd.as_raw());
    serialize(pod::object!(
        SpaTypes::ObjectParamBuffers,
        ParamType::Buffers,
        Property::new(
            SPA_PARAM_BUFFERS_dataType,
            Value::Choice(ChoiceValue::Int(Choice(
                ChoiceFlags::empty(),
                ChoiceEnum::Flags {
                    default: readable,
                    flags: vec![readable],
                },
            ))),
        ),
    ))
}

/// One `EnumFormat` per encoding, MJPEG first: UVC cameras only reach full
/// frame rate at HD sizes compressed.
pub fn enum_formats(request: CameraRequest) -> Result<Vec<Vec<u8>>, CaptureError> {
    [
        format_object(request, MediaSubtype::Mjpg, None),
        format_object(request, MediaSubtype::Raw, Some(VideoFormat::YUY2)),
        format_object(request, MediaSubtype::Raw, Some(VideoFormat::NV12)),
    ]
    .into_iter()
    .map(serialize)
    .collect()
}

fn format_object(
    request: CameraRequest,
    subtype: MediaSubtype,
    raw: Option<VideoFormat>,
) -> pod::Object {
    let size = Rectangle {
        width: request.width,
        height: request.height,
    };
    let rate = Fraction {
        num: request.frames_per_second,
        denom: 1,
    };
    let mut object = pod::object!(
        SpaTypes::ObjectParamFormat,
        ParamType::EnumFormat,
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, subtype),
        pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            size,
            MIN_SIZE,
            MAX_SIZE
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            rate,
            Fraction { num: 0, denom: 1 },
            Fraction { num: 240, denom: 1 }
        ),
    );
    if let Some(format) = raw {
        object
            .properties
            .push(pod::property!(FormatProperties::VideoFormat, Id, format));
    }
    object
}

/// The format a `Format` param fixed, or `None` for one this app cannot decode.
pub fn negotiated(param: &Pod) -> Option<Negotiated> {
    let (media_type, subtype) = pipewire::spa::param::format_utils::parse_format(param).ok()?;
    if media_type != MediaType::Video {
        return None;
    }
    if subtype == MediaSubtype::Mjpg {
        // SAFETY: zeroed is a valid `spa_video_info_mjpg`, which the parser fills from `param`.
        let mut info: pipewire::spa::sys::spa_video_info_mjpg = unsafe { std::mem::zeroed() };
        // SAFETY: `param` is a valid pod for the duration of the call.
        let result = unsafe {
            pipewire::spa::sys::spa_format_video_mjpg_parse(param.as_raw_ptr(), &mut info)
        };
        return (result >= 0).then_some(Negotiated {
            format: CameraFormat::Mjpeg,
            width: info.size.width,
            height: info.size.height,
        });
    }
    let mut info = pipewire::spa::param::video::VideoInfoRaw::new();
    info.parse(param).ok()?;
    let format = match info.format() {
        VideoFormat::YUY2 => CameraFormat::Yuyv,
        VideoFormat::NV12 => CameraFormat::Nv12,
        _ => return None,
    };
    Some(Negotiated {
        format,
        width: info.size().width,
        height: info.size().height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_every_decodable_encoding_and_reads_its_own_offer_back() -> Result<(), CaptureError> {
        let offers = enum_formats(CameraRequest::default())?;
        assert_eq!(offers.len(), 3);
        let mjpeg = Pod::from_bytes(&offers[0]).ok_or(CaptureError::Negotiation)?;
        let (_, subtype) = pipewire::spa::param::format_utils::parse_format(mjpeg)
            .map_err(|_| CaptureError::Negotiation)?;
        assert_eq!(subtype, MediaSubtype::Mjpg);
        Ok(())
    }
}
