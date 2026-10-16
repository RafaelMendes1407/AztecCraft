use super::*;

impl Editor {
    #[cfg(any(test, feature = "mcp"))]
    fn inspect_session_materials(
        &self,
    ) -> Vec<crate::editor_operations::SessionMaterialInspection> {
        let scene = self.history.scene();
        if scene.appearance_incomplete() {
            return Vec::new();
        }
        scene
            .session_materials()
            .iter()
            .zip(scene.session_shading_groups())
            .enumerate()
            .map(
                |(index, (material, group))| crate::editor_operations::SessionMaterialInspection {
                    id: index.to_string(),
                    name: material.name().into(),
                    shading_group: group.name().into(),
                    color: material.color(),
                    editable: index != 0,
                },
            )
            .collect()
    }
    #[cfg(any(test, feature = "mcp"))]
    pub(super) fn current_viewport_error(&self) -> Option<crate::viewport_renderer::ViewportError> {
        use crate::viewport_renderer::ViewportError;
        match &self.renderer {
            None => Some(ViewportError::Uninitialized),
            Some(renderer) if renderer.terminal_error().is_some() => {
                Some(ViewportError::BackendFailed)
            }
            Some(_) => self.viewport_error,
        }
    }
    #[cfg(any(test, feature = "mcp"))]
    pub(super) fn inspect(&self, text_focused: bool) -> Result<SceneInspection, OperationError> {
        crate::editor_operations::check_inspection_size(self.history.scene())?;
        let paths = self
            .history
            .scene()
            .transform_paths()
            .map_err(OperationError::Scene)?;
        let path_bytes = paths
            .iter()
            .try_fold(0usize, |sum, target| {
                sum.checked_add(target.path.len().checked_mul(24)?)
                    .and_then(|sum| sum.checked_add(64))
            })
            .ok_or(OperationError::TooLarge)?;
        let remembered_bytes = |path_len: Option<usize>| {
            path_len
                .map_or(Some(0), |len| {
                    if len > azteccraft::native_scene::MAX_TRANSFORM_DEPTH + 1 {
                        return None;
                    }
                    len.checked_mul(24)?.checked_add(128)
                })
                .ok_or(OperationError::TooLarge)
        };
        let face_bytes = remembered_bytes(self.remembered_face.as_ref().map(|t| t.path.len()))?;
        let vertex_bytes = remembered_bytes(self.remembered_vertex.as_ref().map(|t| t.path.len()))?;
        if path_bytes
            .checked_add(face_bytes)
            .and_then(|bytes| bytes.checked_add(vertex_bytes))
            .is_none_or(|bytes| bytes > crate::editor_operations::MAX_RESULT_BYTES / 2)
        {
            return Err(OperationError::TooLarge);
        }
        let units = self.history.scene().units();
        let linear = match &units.linear {
            LinearUnit::Centimeter => "cm",
            LinearUnit::Millimeter => "mm",
            LinearUnit::Meter => "m",
            LinearUnit::Inch => "in",
            LinearUnit::Foot => "ft",
            LinearUnit::Unsupported(value) => value,
        };
        let angular = match &units.angular {
            AngularUnit::Degree => "deg",
            AngularUnit::Radian => "rad",
            AngularUnit::Unsupported(value) => value,
        };
        if linear
            .len()
            .saturating_add(angular.len())
            .saturating_add(units.time.len())
            > 4096
        {
            return Err(OperationError::TooLarge);
        }
        let nodes = self
            .history
            .scene()
            .nodes()
            .iter()
            .map(|node| {
                Ok(NodeInspection {
                    id: node.id().to_decimal(),
                    transform_id: node.transform_id().to_decimal(),
                    name: node.name().into(),
                    visible: node.local_visibility(),
                    transform: self.transform_input(node.transform())?,
                    authoring_transform: node.transform().into(),
                    shape_ids: self
                        .history
                        .scene()
                        .dag_edges()
                        .iter()
                        .filter(|edge| edge.parent() == Some(node.transform_id()))
                        .filter_map(|edge| {
                            let id = edge.child();
                            (self.history.scene().shape(id).is_ok()
                                || self
                                    .history
                                    .scene()
                                    .curves()
                                    .iter()
                                    .any(|curve| curve.id() == id))
                            .then(|| id.to_decimal())
                        })
                        .collect(),
                    instance_paths: paths
                        .iter()
                        .filter(|target| target.node == node.id())
                        .map(|target| target.path.iter().map(|id| id.to_decimal()).collect())
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, OperationError>>()?;
        let viewport_error = self.current_viewport_error();
        let prepared_view = (!self.history.scene().curves().is_empty())
            .then(|| ViewScene::from_native(self.history.scene(), 256 * 1024 * 1024));
        let gpu_healthy = self.renderer.is_some()
            && viewport_error != Some(crate::viewport_renderer::ViewportError::BackendFailed);
        Ok(SceneInspection {
            scene_token: self.scene_token.to_string(),
            move_tool: self.inspect_move()?,
            nodes,
            retained_source_bytes: self.history.scene().retained_source_bytes(),
            retained_source_records: self.history.scene().retained_source_records(),
            read_only_reason: self
                .history
                .scene()
                .authoring_read_only_reason()
                .map(|reason| reason.to_string()),
            curves: self
                .history
                .scene()
                .curves()
                .iter()
                .map(|curve| {
                    Ok(crate::editor_operations::CurveInspection {
                        id: curve.id().to_decimal(),
                        name: curve.name().into(),
                        degree: curve.degree(),
                        form: curve.form(),
                        control_vertices: curve.control_vertices().len(),
                        knots: curve.knots().len(),
                        domain: curve.domain().map_err(OperationError::Scene)?,
                        read_only: self.history.scene().authoring_read_only_reason().is_some(),
                        draw_available: prepared_view.as_ref().is_some_and(|prepared| {
                            prepared.as_ref().is_ok_and(|view| {
                                view.curve_instances()
                                    .iter()
                                    .any(|instance| instance.shape() == curve.id())
                            })
                        }),
                    })
                })
                .collect::<Result<Vec<_>, OperationError>>()?,
            shapes: self
                .history
                .scene()
                .shapes()
                .iter()
                .map(|shape| {
                    Ok(ShapeInspection {
                        id: shape.id().to_decimal(),
                        name: shape.name().into(),
                        points: shape.mesh().point_count(),
                        edges: shape.mesh().edges().len(),
                        faces: shape.mesh().faces().len(),
                        smoothing_known: shape
                            .mesh()
                            .edges()
                            .iter()
                            .all(|edge| edge.smooth().is_some()),
                        source_appearance: self
                            .history
                            .scene()
                            .shape_appearance_readiness(shape.id())
                            .map_err(OperationError::Scene)?,
                        session_material: self
                            .history
                            .scene()
                            .session_material_id(shape)
                            .map(|id| id.index().to_string()),
                        material: match self.history.scene().material_for_shape(shape.id()) {
                            Ok(value) => Some(crate::editor_operations::MaterialInspection {
                                shader: match value.shader {
                                    azteccraft::native_scene::MaterialShader::Phong => "phong",
                                },
                                diffuse: value.diffuse,
                                shared_users: value.shared_users,
                                editing_available: value.editing_available,
                                rendering_available: value.rendering_available,
                            }),
                            Err(azteccraft::native_scene::NativeError::UnsupportedEdit) => None,
                            Err(error) => return Err(OperationError::Scene(error)),
                        },
                    })
                })
                .collect::<Result<Vec<_>, OperationError>>()?,
            session_materials: self.inspect_session_materials(),
            units: UnitsInspection {
                linear: linear.into(),
                angular: angular.into(),
                time: units.time.clone(),
            },
            save_available: self.files.save_available(),
            save_as_available: self.files.save_as_available(),
            save_reconciliation_required: self.files.reconciliation_required(),
            document_origin: self.files.document_origin(),
            has_file: self.files.has_file(),
            closing: self.files.is_closing(),
            viewport_backend: if self.renderer.is_some() {
                "wgpu_geometry_preview"
            } else {
                "uninitialized"
            },
            viewport_error,
            grid_visible: self.grid_visible,
            shading: self.shading,
            face_centres_hidden: self.face_centres_hidden,
            sidebar_panels: SidebarPanelsInspection {
                channel_box: self.sidebar_panel_visible(SidebarPanel::ChannelBox),
                attribute_editor: self.sidebar_panel_visible(SidebarPanel::AttributeEditor),
            },
            attribute_editor_tab: self
                .sidebar_panel_visible(SidebarPanel::AttributeEditor)
                .then(|| self.effective_attribute_editor_tab())
                .flatten()
                .map(|tab| match tab {
                    AttributeEditorTab::Transform(id) => AttributeEditorTabInspection {
                        kind: "transform",
                        id: id.to_decimal(),
                    },
                    AttributeEditorTab::Shape(id) => AttributeEditorTabInspection {
                        kind: "shape",
                        id: id.to_decimal(),
                    },
                    AttributeEditorTab::Material(id) => AttributeEditorTabInspection {
                        kind: "material",
                        id: id.to_decimal(),
                    },
                }),
            depth_complete: gpu_healthy,
            clipping_complete: gpu_healthy,
            appearance_incomplete: self.history.scene().appearance_incomplete(),
            source_appearance: self.history.scene().appearance_readiness(),
            pending_action: self.files.pending_inspection(),
            selected_node: self.selected.map(NodeId::to_decimal),
            selected_instance_path: self
                .selected_path
                .iter()
                .map(|id| id.to_decimal())
                .collect(),
            selection_mode: self.selection_mode,
            remembered_face: self
                .remembered_face
                .as_ref()
                .map(|target| FaceSelectionTarget {
                    node_id: target.transform.to_decimal(),
                    shape_id: target.shape.to_decimal(),
                    instance_path: target.path.iter().map(|id| id.to_decimal()).collect(),
                    face_index: target.face_index,
                }),
            remembered_vertex: self.remembered_vertex.as_ref().map(|target| {
                VertexSelectionTarget {
                    node_id: target.transform.to_decimal(),
                    shape_id: target.shape.to_decimal(),
                    instance_path: target.path.iter().map(|id| id.to_decimal()).collect(),
                    vertex_index: target.vertex_index,
                }
            }),
            face_selection_policy: FaceSelectionPolicy::VisibleCentersOnly,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            dirty: self.is_dirty(),
            has_draft: self.has_draft(),
            native_input_pending: self.native_input_pending,
            text_focused,
            modal: self.files.is_modal(),
            camera: CameraInspection {
                target: self.camera.target().to_array(),
                eye: self.camera.eye().to_array(),
                distance: self.camera.distance(),
            },
        })
    }
}
