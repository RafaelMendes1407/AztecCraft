use super::Editor;
use azteccraft::{document, history::History, scene::Scene};
use eframe::egui::{self, Key};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct FileState {
    path: Option<PathBuf>,
    saved: Scene,
    pending: Option<FileAction>,
    close_approved: bool,
}

enum FileAction {
    New,
    Open { scene: Scene, path: PathBuf },
    Close,
}

impl FileState {
    pub(super) fn is_modal(&self) -> bool {
        self.pending.is_some()
    }
}

impl Editor {
    pub(super) fn document_title(&self) -> String {
        let name = self
            .files
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into());
        format!(
            "AztecCraft | {name}{}",
            if self.is_dirty() { " *" } else { "" }
        )
    }

    fn is_dirty(&self) -> bool {
        self.history.scene() != &self.files.saved
            || self
                .selected
                .and_then(|id| self.history.scene().node(id).ok())
                .is_some_and(|node| {
                    let t = node.transform();
                    let values = [
                        t.translation.x,
                        t.translation.y,
                        t.translation.z,
                        t.rotation_degrees.x,
                        t.rotation_degrees.y,
                        t.rotation_degrees.z,
                        t.scale.x,
                        t.scale.y,
                        t.scale.z,
                    ];
                    node.name() != self.name
                        || values
                            .iter()
                            .zip(&self.channels)
                            .any(|(value, draft)| draft.parse::<f32>().ok() != Some(*value))
                })
    }

    fn request_file_action(&mut self, action: FileAction) {
        if self.files.is_modal() || !self.commit_pending() {
            return;
        }
        if self.is_dirty() {
            self.files.pending = Some(action);
        } else {
            self.apply_file_action(action);
        }
    }

    pub(super) fn request_new(&mut self) {
        self.request_file_action(FileAction::New);
    }
    pub(super) fn request_close(&mut self) {
        self.request_file_action(FileAction::Close);
    }

    pub(super) fn open_dialog(&mut self) {
        if self.files.is_modal() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open AztecCraft scene")
            .add_filter("AztecCraft scene", &["aztec"])
            .pick_file()
        {
            self.open_path(&path);
        }
    }

    fn open_path(&mut self, path: &Path) {
        match document::load(path) {
            Ok(scene) => self.request_file_action(FileAction::Open {
                scene,
                path: path.into(),
            }),
            Err(error) => {
                self.status = error.to_string();
                self.error = true;
            }
        }
    }

    pub(super) fn save_dialog(&mut self) -> bool {
        if !self.commit_pending() {
            return false;
        }
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save As a new AztecCraft scene")
            .add_filter("AztecCraft scene", &["aztec"])
            .set_file_name("Untitled.aztec");
        if let Some(parent) = self.files.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(parent);
        }
        let Some(mut path) = dialog.save_file() else {
            return false;
        };
        if path.extension().is_none() {
            path.set_extension("aztec");
        }
        self.save_path(&path)
    }

    fn save_path(&mut self, path: &Path) -> bool {
        if !self.commit_pending() {
            return false;
        }
        match document::save_new(path, self.history.scene()) {
            Ok(()) => {
                self.files.path = Some(path.into());
                self.files.saved = self.history.scene().clone();
                self.status = "Scene saved".into();
                self.error = false;
                true
            }
            Err(error) => {
                self.status = error.to_string();
                self.error = true;
                false
            }
        }
    }

    fn apply_file_action(&mut self, action: FileAction) {
        match action {
            FileAction::Close => self.files.close_approved = true,
            FileAction::New => self.replace_scene(Scene::default(), None),
            FileAction::Open { scene, path } => self.replace_scene(scene, Some(path)),
        }
    }

    fn replace_scene(&mut self, scene: Scene, path: Option<PathBuf>) {
        self.files.saved = scene.clone();
        self.files.path = path;
        self.history = History::new(scene);
        self.selected = None;
        self.name.clear();
        self.channels = Default::default();
        self.camera = Default::default();
        self.status = "Scene ready".into();
        self.error = false;
    }

    fn discard_pending(&mut self) {
        if let Some(action) = self.files.pending.take() {
            self.apply_file_action(action);
        }
    }

    pub(super) fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.files.close_approved {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_close();
        }
        if self.files.close_approved {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(super) fn file_modal(&mut self, ctx: &egui::Context) {
        if self.files.close_approved {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.files.is_modal() {
            return;
        }
        let mut save = false;
        let mut discard = false;
        let mut cancel = ctx.input(|i| i.key_pressed(Key::Escape));
        egui::Modal::new(egui::Id::new("unsaved_scene")).show(ctx, |ui| {
            ui.heading("Save changes to this scene?");
            ui.label("Discarding changes cannot be undone.");
            ui.label("Save As creates a new file and preserves existing files.");
            ui.horizontal(|ui| {
                save = ui.button("Save As...").clicked();
                discard = ui.button("Discard").clicked();
                cancel |= ui.button("Cancel").clicked();
            });
        });
        if cancel {
            self.files.pending = None;
        } else if discard || (save && self.save_dialog()) {
            self.discard_pending();
            if self.files.close_approved {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azteccraft::{history::Command, scene::Primitive};

    fn ui_frame(
        editor: &mut Editor,
        ctx: &egui::Context,
        input: egui::RawInput,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(input, |ui| editor.render(ui));
        output.textures_delta.clear();
        output
    }

    fn escape_modal(editor: &mut Editor) {
        let ctx = egui::Context::default();
        ui_frame(editor, &ctx, egui::RawInput::default());
        ui_frame(
            editor,
            &ctx,
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
        );
    }

    #[test]
    fn save_open_occupied_path_and_malformed_open_use_real_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.aztec");
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let cube = editor.selected.unwrap();
        editor.channels[0] = "2".into();
        assert!(editor.save_path(&path));
        assert!(!editor.is_dirty());
        let saved = std::fs::read(&path).unwrap();
        editor.channels[0] = "3".into();
        assert!(!editor.save_path(&path));
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert!(editor.is_dirty());
        editor.channels[0] = "4".into();
        let before = editor.history.scene().clone();
        let draft = editor.channels.clone();
        let bad = dir.path().join("bad.aztec");
        std::fs::write(&bad, b"{").unwrap();
        editor.open_path(&bad);
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.channels, draft);
        assert_eq!(editor.selected, Some(cube));
        assert!(editor.history.can_undo());
        editor.open_path(&path);
        assert!(editor.files.is_modal());
        editor.discard_pending();
        assert_eq!(
            editor
                .history
                .scene()
                .node(cube)
                .unwrap()
                .transform()
                .translation
                .x,
            2.0
        );
        assert!(!editor.history.can_undo());
        assert!(!editor.is_dirty());
    }

    #[test]
    fn new_and_close_cancel_preserve_scene_and_discard_is_explicit() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let before = editor.history.scene().clone();
        editor.request_new();
        assert!(editor.files.is_modal());
        escape_modal(&mut editor);
        assert!(!editor.files.is_modal());
        assert_eq!(editor.history.scene(), &before);
        assert!(editor.history.can_undo());
        editor.request_close();
        assert!(editor.files.is_modal());
        escape_modal(&mut editor);
        assert!(!editor.files.is_modal());
        assert!(!editor.files.close_approved);
        assert_eq!(editor.history.scene(), &before);
        editor.request_new();
        editor.discard_pending();
        assert!(editor.history.scene().nodes().is_empty());
        assert!(!editor.is_dirty());
        editor.request_close();
        assert!(editor.files.close_approved);
    }

    #[test]
    fn native_close_event_is_cancelled_until_unsaved_decision() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ui_frame(&mut editor, &ctx, input);
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::CancelClose))
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        );
        assert!(editor.files.is_modal());
        escape_modal(&mut editor);
        assert!(!editor.files.close_approved);
        assert_eq!(editor.history.scene().nodes().len(), 1);
    }

    #[test]
    fn invalid_draft_blocks_new_close_and_save_without_resetting_editor() {
        let dir = tempfile::tempdir().unwrap();
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        editor.name = "".into();
        let before = editor.history.scene().clone();
        editor.request_new();
        editor.request_close();
        assert!(!editor.files.is_modal());
        assert!(!editor.files.close_approved);
        assert!(!editor.save_path(&dir.path().join("invalid.aztec")));
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.name, "");
        assert!(editor.error);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        editor.sync_fields();
        editor.execute(Command::Rename {
            id: editor.selected.unwrap(),
            name: "changed".into(),
        });
        assert!(editor.is_dirty());
    }
}
