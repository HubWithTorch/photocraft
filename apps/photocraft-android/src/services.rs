//! Platform-independent image services, tested on the host as well as Android.
use photocraft_ui_egui::Services;
pub(crate) fn services() -> Services {
    Services {
        import: Some(Box::new(|name, bytes, depth| {
            photocraft_io::import_with_svg_group_depth(name, bytes, depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        export: Some(Box::new(|doc, name, settings| {
            let mut opts = photocraft_io::ExportOptions::default();
            if let Some(q) = settings.jpeg_quality {
                opts.encode.jpeg_quality = q;
            }
            opts.encode.webp_lossless = settings.webp_lossless;
            if let Some(q) = settings.webp_quality {
                opts.encode.webp_quality = q;
            }
            opts.tiff_layers = settings.tiff_layers;
            opts.xmp = if settings.xmp_all { photocraft_io::XmpEmbed::All } else { photocraft_io::XmpEmbed::None };
            opts.tga_bits = settings.tga_bits;
            photocraft_io::export(doc, name, &opts).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())
        })),
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_ui_egui::{FileDialogAnswer, FileDialogRequest, PhotocraftApp};
    use std::sync::{Arc, Mutex};
    #[test]
    fn picker_open_export_and_repeated_save_round_trip() {
        let mut services = services();
        let mut session = photocraft_engine::Session::new();
        session.execute("file.new", serde_json::json!({"width": 8, "height": 8})).unwrap();
        let doc = session.active().unwrap().doc.clone();
        let (png, _) = services.export.as_ref().unwrap()(&doc, "test.png", &Default::default()).unwrap();
        let data = png.clone();
        services.file_dialog = Some(Box::new(move |request, _, reply| {
            reply.send(Some(match request {
                FileDialogRequest::Open { .. } => FileDialogAnswer::ContentsMany(vec![("test.png".into(), data.clone())]),
                FileDialogRequest::Save { .. } => FileDialogAnswer::SaveTo("android-saf/0/test.png".into()),
            }));
        }));
        let writes = Arc::new(Mutex::new(Vec::new()));
        let capture = writes.clone();
        services.write = Some(Box::new(move |name, bytes| {
            capture.lock().unwrap().push((name.to_owned(), bytes.to_vec()));
            Ok(())
        }));
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        let ctx = egui::Context::default();
        app.open_dialog_file().unwrap();
        app.poll_file_dialog(&ctx, None);
        app.poll_file_dialog(&ctx, None);
        assert_eq!(app.session.documents().len(), 1);
        app.save_as(None).unwrap();
        app.poll_file_dialog(&ctx, None);
        app.poll_file_dialog(&ctx, None);
        app.save_as(Some("android-saf/0/test.png".into())).unwrap();
        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 2);
        assert!(writes.iter().all(|(name, bytes)| name == "android-saf/0/test.png" && bytes.starts_with(b"\x89PNG")));
    }
}
