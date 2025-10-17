use std::io::{Cursor, Error};
use std::sync::Arc;

use ab_glyph::{FontRef, PxScale};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut, text_size};
use imageproc::image;
use imageproc::image::{ImageFormat, Rgb};
use imageproc::rect::Rect;
use log::error;
use tokio::sync::Mutex;

use crate::config::{FONT, SERVER_CONFIG};
use crate::config::time::{convert_time_local, get_current_millis};
use crate::database::{Database, generate_uuid};
use crate::http::request::Request;
use crate::results;
use crate::ip::ip_info::IPInfo;
use crate::results::TelemetryData;

pub async fn record_result (request : &Request, database : &mut Arc<Mutex<dyn Database + Send>>) -> std::io::Result<String> {
    let default = "".to_string();
    let mut ip_address = request.remote_addr.to_string();
    let mut isp_info = request.form_data.get("ispinfo").unwrap_or(&default).clone();
    let extra = request.form_data.get("extra").unwrap_or(&default);
    let ua = request.headers.get("User-Agent").unwrap_or(&default);
    let lang = request.headers.get("Accept-Language").unwrap_or(&default);
    let dl = request.form_data.get("dl").unwrap_or(&default);
    let ul = request.form_data.get("ul").unwrap_or(&default);
    let ping = request.form_data.get("ping").unwrap_or(&default);
    let jitter = request.form_data.get("jitter").unwrap_or(&default);
    let mut log = request.form_data.get("log").unwrap_or(&default).clone();
    let uuid = generate_uuid();

    let config = SERVER_CONFIG.get().unwrap();
    if config.redact_ip_addresses {
        ip_address = "0.0.0.0".to_string();
        results::redact_hostname(&mut isp_info,"\"hostname\":\"REDACTED\"");
        results::redact_all_ips(&mut isp_info,"0.0.0.0");
        results::redact_hostname(&mut log,"\"hostname\":\"REDACTED\"");
        results::redact_all_ips(&mut log,"0.0.0.0");
    }

    let mut database = database.lock().await;
    let insert_db = database.insert(TelemetryData {
        ip_address,
        isp_info: isp_info.to_string(),
        extra: extra.to_string(),
        user_agent: ua.to_string(),
        lang: lang.to_string(),
        download: dl.to_string(),
        upload: ul.to_string(),
        ping: ping.to_string(),
        jitter: jitter.to_string(),
        log: log.to_string(),
        uuid: uuid.to_string(),
        timestamp: get_current_millis(),
    });
    match insert_db {
        Ok(_) => {
            Ok(uuid)
        }
        Err(e) => {
            Err(Error::other(e))
        }
    }
}

struct ImageTheme {
    background : Rgb<u8>,
    text_head : Rgb<u8>,
    text_value : Rgb<u8>,
    text_unit : Rgb<u8>,
}
fn get_theme(is_dark : bool) -> ImageTheme {
    if is_dark {
        ImageTheme {
            background: Rgb([42,42,42]),
            text_head: Rgb([255,255,255]),
            text_value: Rgb([120,166,240]),
            text_unit: Rgb([174,174,174]),
        }
    } else { 
        ImageTheme {
            background: Rgb([255,255,255]),
            text_head: Rgb([0,0,0]),
            text_value: Rgb([96,96,170]),
            text_unit: Rgb([110,110,110]),
        }
    }
}
pub fn draw_result (data : &TelemetryData) -> Vec<u8> {

    fn cal_text_size (font : &FontRef,text : &str,scale : f32) -> (u32,u32) {
        text_size(PxScale::from(scale),font,text)
    }

    //scales
    let d_u_name_scale: f32 = 32.0;
    let d_u_value_scale: f32 = 60.0;
    let ping_jitter_name_scale: f32 = 28.0;
    let ping_jitter_value_scale: f32 = 52.0;
    let unit_scale: f32 = 20.0;
    let footer_scale: f32 = 17.0;
    let v_padding: u32 = 10;
    let h_padding: u32 = 24;
    let unit_padding: u32 = 6;

    //init
    let mut img = image::RgbImage::new(500,286);

    //load font
    let font = FONT.get().unwrap();

    //labels
    let l_ping = "Ping";
    let l_jitter = "Jitter";
    let l_ms = "ms";
    let l_dl = "Download";
    let l_ul = "Upload";
    let l_mbps = "Mbps";
    let l_watermark = "LibreSpeed";

    //text sizes
    let ping_text_size = cal_text_size(font, l_ping, ping_jitter_name_scale);
    let ping_value_text_size = cal_text_size(font, &data.ping, ping_jitter_value_scale);
    let jitter_text_size = cal_text_size(font, l_jitter, ping_jitter_name_scale);
    let jitter_value_text_size = cal_text_size(font, &data.jitter, ping_jitter_value_scale);
    let ms_text_size = cal_text_size(font, l_ms, unit_scale);
    let download_text_size = cal_text_size(font, l_dl, d_u_name_scale);
    let download_value_text_size = cal_text_size(font, &data.download, d_u_value_scale);
    let upload_text_size = cal_text_size(font, l_ul, d_u_name_scale);
    let upload_value_text_size = cal_text_size(font, &data.upload, d_u_value_scale);
    let mbps_text_size = cal_text_size(font, l_mbps, unit_scale);
    let watermark_text_size = cal_text_size(font, l_watermark, footer_scale);


    //drawing ....
    //background
    let config = SERVER_CONFIG.get().unwrap();
    let theme = get_theme(config.result_image_theme == "dark");

    draw_filled_rect_mut(&mut img,Rect::at(0,0).of_size(500,286),theme.background);

    let width_quarter = img.width() / 4;
    let width_3_quarter = width_quarter * 3;

    //ping
    let mut x = width_quarter - (ping_text_size.0 / 2) + h_padding; // ping label
    let mut y = v_padding; // ping label
    draw_text_mut(&mut img, theme.text_head, x as i32, y as i32, PxScale::from(ping_jitter_name_scale), font, l_ping); // ping label

    x = width_quarter - (ping_value_text_size.0 / 2) + h_padding - (ms_text_size.0 / 2); // ping value
    y = ping_text_size.1 + (v_padding * 2); // ping value
    draw_text_mut(&mut img, theme.text_value, x as i32, y as i32, PxScale::from(ping_jitter_value_scale), font, &data.ping); // ping value

    x = width_quarter + (ping_value_text_size.0 / 2) + unit_padding + h_padding - (ms_text_size.0 / 2); // ping unit
    y = ping_text_size.1 + (v_padding * 2) + ping_value_text_size.1 - ms_text_size.1; // ping unit
    draw_text_mut(&mut img, theme.text_unit, x as i32, y as i32, PxScale::from(unit_scale), font, l_ms); // ping unit

    //jitter
    x = width_3_quarter - (jitter_text_size.0 / 2) - h_padding; // jitter label
    y = v_padding; // jitter value
    draw_text_mut(&mut img, theme.text_head, x as i32, y as i32, PxScale::from(ping_jitter_name_scale), font, l_jitter); // jitter value

    x = width_3_quarter - (jitter_value_text_size.0 / 2) - h_padding - (ms_text_size.0 / 2); // jitter value
    y = jitter_text_size.1 + (v_padding * 2); // jitter value
    draw_text_mut(&mut img, theme.text_value, x as i32, y as i32, PxScale::from(ping_jitter_value_scale), font, &data.jitter); // jitter value

    x = width_3_quarter + (jitter_value_text_size.0 / 2) + unit_padding - h_padding - (ms_text_size.0 / 2);// jitter unit
    y = jitter_text_size.1 + (v_padding * 2) + jitter_value_text_size.1 - ms_text_size.1;// jitter unit
    draw_text_mut(&mut img, theme.text_unit, x as i32, y as i32, PxScale::from(unit_scale), font, l_ms);// jitter unit

    //download
    x = width_quarter - (download_text_size.0 / 2) + h_padding; // download label
    y = ping_text_size.1 + ping_value_text_size.1 + (v_padding * 6); // download label
    draw_text_mut(&mut img, theme.text_head, x as i32, y as i32, PxScale::from(d_u_name_scale), font, l_dl); // download label

    x = width_quarter - (download_value_text_size.0 / 2) + h_padding;// download value
    y = ping_text_size.1 + ping_value_text_size.1 + download_text_size.1 + (v_padding * 7);// download value
    draw_text_mut(&mut img, theme.text_value, x as i32, y as i32, PxScale::from(d_u_value_scale), font, &data.download);// download value

    x = width_quarter - (mbps_text_size.0 / 2) + h_padding;//download unit
    y = ping_text_size.1 + (unit_padding * 2) + ping_value_text_size.1 + download_text_size.1 + download_value_text_size.1 + (v_padding * 8);//download unit
    draw_text_mut(&mut img, theme.text_unit, x as i32, y as i32, PxScale::from(unit_scale), font, l_mbps);//download unit

    //upload
    x = width_3_quarter - (upload_text_size.0 / 2) - h_padding; // upload label
    y = jitter_text_size.1 + jitter_value_text_size.1 + (v_padding * 6); // upload label
    draw_text_mut(&mut img, theme.text_head, x as i32, y as i32, PxScale::from(d_u_name_scale), font, l_ul); // upload label

    x = width_3_quarter - (upload_value_text_size.0 / 2) - h_padding;// upload value
    y = jitter_text_size.1 + jitter_value_text_size.1 + upload_text_size.1 + (v_padding * 7);// upload value
    draw_text_mut(&mut img, theme.text_value, x as i32, y as i32, PxScale::from(d_u_value_scale), font, &data.upload);// upload value

    x = width_3_quarter - (mbps_text_size.0 / 2) - h_padding;//upload unit
    y = jitter_text_size.1 + (unit_padding * 2) + jitter_value_text_size.1 + upload_text_size.1 + upload_value_text_size.1 + (v_padding * 8);//upload unit
    draw_text_mut(&mut img, theme.text_unit, x as i32, y as i32, PxScale::from(unit_scale), font, l_mbps);//upload unit

    //isp_info
    x = unit_padding;
    y = img.height() - (watermark_text_size.1 * 2) - (unit_padding * 5);
    let isp_info : IPInfo = serde_json::from_str(&data.isp_info).unwrap();
    draw_text_mut(&mut img,theme.text_head,x as i32,y as i32,PxScale::from(footer_scale),font,&isp_info.processedString);
    drop(isp_info);

    //footer divider
    let divider_y = (img.height() - watermark_text_size.1 - (unit_padding * 3)) as f32;
    draw_line_segment_mut(&mut img, (0.0, divider_y), (500f32, divider_y), theme.text_unit);

    //watermark
    x = img.width() - watermark_text_size.0 - unit_padding;
    y = img.height() - watermark_text_size.1 - (unit_padding * 2);
    draw_text_mut(&mut img,theme.text_unit,x as i32,y as i32,PxScale::from(footer_scale),font,l_watermark);

    //time
    x = unit_padding;
    y = img.height() - watermark_text_size.1 - (unit_padding * 2);
    let time = convert_time_local(data.timestamp);
    draw_text_mut(&mut img,theme.text_unit,x as i32,y as i32,PxScale::from(footer_scale),font,&time);

    let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    if let Err(e) = img.write_to(&mut buffer, ImageFormat::Jpeg) {
        error!("Image writer buffer error : {e}")
    }
    img.fill(0);
    drop(img);

    buffer.into_inner()
}