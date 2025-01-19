use anyhow::Context;
use windows::{
    Foundation::Numerics::Matrix3x2,
    Win32::Graphics::Direct2D::{
        CLSID_D2D1AlphaMask, CLSID_D2D1Composite, CLSID_D2D1GaussianBlur, CLSID_D2D1Opacity,
        CLSID_D2D1Shadow, CLSID_D2D12DAffineTransform, Common::D2D1_COMPOSITE_MODE_SOURCE_OVER,
        D2D1_2DAFFINETRANSFORM_PROP_TRANSFORM_MATRIX, D2D1_DIRECTIONALBLUR_OPTIMIZATION_SPEED,
        D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION, D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION,
        D2D1_INTERPOLATION_MODE_LINEAR, D2D1_OPACITY_PROP_OPACITY, D2D1_PROPERTY_TYPE_ENUM,
        D2D1_PROPERTY_TYPE_FLOAT, D2D1_PROPERTY_TYPE_MATRIX_3X2,
        D2D1_SHADOW_PROP_BLUR_STANDARD_DEVIATION, D2D1_SHADOW_PROP_OPTIMIZATION, ID2D1CommandList,
        ID2D1Effect,
    },
};

use crate::render_resources::RenderResources;

use super::{
    EffectsConfig,
    engine::{EffectEngine, EffectKind},
};

#[derive(Debug, Default, Clone)]
pub struct EffectManager {
    active: Vec<EffectEngine>,
    inactive: Vec<EffectEngine>,
    active_command_list: Option<ID2D1CommandList>,
    inactive_command_list: Option<ID2D1CommandList>,
}

impl EffectManager {
    pub fn active(&self) -> &Vec<EffectEngine> {
        &self.active
    }

    pub fn inactive(&self) -> &Vec<EffectEngine> {
        &self.inactive
    }

    pub fn is_enabled(&self) -> bool {
        !self.active.is_empty() || !self.inactive.is_empty()
    }

    pub fn create_command_list(
        &mut self,
        render_resources: &RenderResources,
    ) -> anyhow::Result<()> {
        let d2d_context = render_resources.d2d_context()?;
        let border_bitmap = render_resources.border_bitmap()?;
        let mask_bitmap = render_resources.mask_bitmap()?;

        let create_single_list =
            |effect_params_vec: &Vec<EffectEngine>| -> anyhow::Result<ID2D1CommandList> {
                unsafe {
                    // Open a command list to record draw operations
                    let command_list = d2d_context
                        .CreateCommandList()
                        .context("d2d_context.CreateCommandList()")?;

                    // Set the command list as the target so we can begin recording
                    d2d_context.SetTarget(&command_list);

                    let mut effects_vec: Vec<ID2D1Effect> = Vec::new();

                    for effect_params in effect_params_vec.iter() {
                        let effect = match effect_params.kind {
                            EffectKind::Glow => {
                                let blur_effect = d2d_context
                                    .CreateEffect(&CLSID_D2D1GaussianBlur)
                                    .context("blur_effect")?;
                                blur_effect.SetInput(0, border_bitmap, false);
                                blur_effect
                                    .SetValue(
                                        D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 as u32,
                                        D2D1_PROPERTY_TYPE_FLOAT,
                                        &effect_params.standard_deviation.to_le_bytes(),
                                    )
                                    .context("blur_effect.SetValue() std deviation")?;
                                blur_effect
                                    .SetValue(
                                        D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION.0 as u32,
                                        D2D1_PROPERTY_TYPE_ENUM,
                                        &D2D1_DIRECTIONALBLUR_OPTIMIZATION_SPEED.0.to_le_bytes(),
                                    )
                                    .context("blur_effect.SetValue() optimization")?;

                                blur_effect
                            }
                            EffectKind::Shadow => {
                                let shadow_effect = d2d_context
                                    .CreateEffect(&CLSID_D2D1Shadow)
                                    .context("shadow_effect")?;
                                shadow_effect.SetInput(0, border_bitmap, false);
                                shadow_effect
                                    .SetValue(
                                        D2D1_SHADOW_PROP_BLUR_STANDARD_DEVIATION.0 as u32,
                                        D2D1_PROPERTY_TYPE_FLOAT,
                                        &effect_params.standard_deviation.to_le_bytes(),
                                    )
                                    .context("shadow_effect.SetValue() std deviation")?;
                                shadow_effect
                                    .SetValue(
                                        D2D1_SHADOW_PROP_OPTIMIZATION.0 as u32,
                                        D2D1_PROPERTY_TYPE_ENUM,
                                        &D2D1_DIRECTIONALBLUR_OPTIMIZATION_SPEED.0.to_le_bytes(),
                                    )
                                    .context("shadow_effect.SetValue() optimization")?;

                                shadow_effect
                            }
                        };

                        let effect_with_opacity = d2d_context
                            .CreateEffect(&CLSID_D2D1Opacity)
                            .context("effect_with_opacity")?;
                        effect_with_opacity.SetInput(
                            0,
                            &effect
                                .GetOutput()
                                .context("could not get _ effect output")?,
                            false,
                        );
                        effect_with_opacity
                            .SetValue(
                                D2D1_OPACITY_PROP_OPACITY.0 as u32,
                                D2D1_PROPERTY_TYPE_FLOAT,
                                &effect_params.opacity.to_le_bytes(),
                            )
                            .context("effect_with_opacity.SetValue()")?;

                        let effect_with_opacity_translation = d2d_context
                            .CreateEffect(&CLSID_D2D12DAffineTransform)
                            .context("effect_with_opacity_translation")?;
                        effect_with_opacity_translation.SetInput(
                            0,
                            &effect_with_opacity
                                .GetOutput()
                                .context("could not get effect_with_opacity output")?,
                            false,
                        );
                        let translation_matrix = Matrix3x2::translation(
                            effect_params.translation.x,
                            effect_params.translation.y,
                        );
                        let translation_matrix_bytes: &[u8] = std::slice::from_raw_parts(
                            &translation_matrix as *const Matrix3x2 as *const u8,
                            size_of::<Matrix3x2>(),
                        );
                        effect_with_opacity_translation
                            .SetValue(
                                D2D1_2DAFFINETRANSFORM_PROP_TRANSFORM_MATRIX.0 as u32,
                                D2D1_PROPERTY_TYPE_MATRIX_3X2,
                                translation_matrix_bytes,
                            )
                            .context("effect_with_opacity_translation.SetValue()")?;

                        effects_vec.push(effect_with_opacity_translation);
                    }

                    // Create a composite effect and link it to the above effects
                    // TODO: if no effects are selected, I will get an invalid graph config error
                    let composite_effect = d2d_context
                        .CreateEffect(&CLSID_D2D1Composite)
                        .context("composite_effect")?;
                    composite_effect
                        .SetInputCount(effects_vec.len() as u32 + 1)
                        .context("could not set composite effect input count")?;

                    for (index, effect) in effects_vec.iter().enumerate() {
                        composite_effect.SetInput(
                            index as u32,
                            &effect
                                .GetOutput()
                                .context(format!("could not get effect output: {}", index))?,
                            false,
                        );
                    }
                    composite_effect.SetInput(effects_vec.len() as u32, border_bitmap, false);

                    // Create an alpha mask effect to mask out the inner rect
                    let mask_effect = d2d_context
                        .CreateEffect(&CLSID_D2D1AlphaMask)
                        .context("mask_effect")?;
                    mask_effect.SetInput(
                        0,
                        &composite_effect
                            .GetOutput()
                            .context("could not get composite output")?,
                        false,
                    );
                    mask_effect.SetInput(1, mask_bitmap, false);

                    // Begin recording commands to the command list
                    d2d_context.BeginDraw();
                    d2d_context.Clear(None);

                    // Record the composite effect
                    d2d_context.DrawImage(
                        &mask_effect
                            .GetOutput()
                            .context("could not get mask output")?,
                        None,
                        None,
                        D2D1_INTERPOLATION_MODE_LINEAR,
                        D2D1_COMPOSITE_MODE_SOURCE_OVER,
                    );

                    d2d_context.EndDraw(None, None)?;

                    // Close the command list to tell it we are done recording
                    command_list.Close().context("command_list.Close()")?;

                    Ok(command_list)
                }
            };

        let active_command_list =
            create_single_list(&self.active).context("active_command_list")?;
        let inactive_command_list =
            create_single_list(&self.inactive).context("inactive_command_list")?;

        self.active_command_list = Some(active_command_list);
        self.inactive_command_list = Some(inactive_command_list);

        Ok(())
    }

    pub fn active_command_list(&self) -> anyhow::Result<&ID2D1CommandList> {
        self.active_command_list
            .as_ref()
            .context("could not get active_command_list")
    }

    pub fn inactive_command_list(&self) -> anyhow::Result<&ID2D1CommandList> {
        self.inactive_command_list
            .as_ref()
            .context("could not get inactive_command_list")
    }
}

impl TryFrom<EffectsConfig> for EffectManager {
    type Error = anyhow::Error;

    fn try_from(value: EffectsConfig) -> Result<Self, Self::Error> {
        let active: Vec<EffectEngine> = value
            .active
            .iter()
            .cloned() // Convert &EffectConfig to EffectConfig
            .map(EffectEngine::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let inactive: Vec<EffectEngine> = value
            .inactive
            .iter()
            .cloned() // Convert &EffectConfig to EffectConfig
            .map(EffectEngine::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        if value.enabled {
            Ok(EffectManager {
                active,
                inactive,
                ..Default::default()
            })
        } else {
            Ok(EffectManager::default())
        }
    }
}
