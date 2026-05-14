use crate::core::config::models::ActiveArea;

/// Pure geometric calculation logic for the tablet active area visualization.
/// This structure is independent of egui and can be unit tested.
#[derive(Debug, Clone)]
pub struct ActiveAreaGeometry {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub aa_center_x: f32,
    pub aa_center_y: f32,
    pub points: Vec<(f32, f32)>,
    pub osu_playfield_points: Option<Vec<(f32, f32)>>,
}

impl ActiveAreaGeometry {
    /// Calculates the geometry for the active area visualization.
    ///
    /// # Arguments
    /// * `phys_w`, `phys_h` - Physical dimensions of the tablet in mm.
    /// * `viz_w`, `viz_h` - Dimensions of the visualization area in pixels.
    /// * `viz_center_x`, `viz_center_y` - Center of the visualization area in pixels.
    /// * `active_area` - The active area configuration.
    /// * `target_w`, `target_h` - The target screen resolution (for osu! playfield).
    /// * `show_osu_playfield` - Whether to calculate the osu! playfield points.
    #[allow(clippy::too_many_arguments)]
    pub fn calculate(
        phys_w: f32,
        phys_h: f32,
        viz_w: f32,
        viz_h: f32,
        viz_center_x: f32,
        viz_center_y: f32,
        active_area: &ActiveArea,
        target_w: f32,
        target_h: f32,
        show_osu_playfield: bool,
    ) -> Self {
        let scale = (viz_w / phys_w).min(viz_h / phys_h) * 0.8;
        let draw_w = phys_w * scale;
        let draw_h = phys_h * scale;
        let offset_x = viz_center_x - draw_w / 2.0;
        let offset_y = viz_center_y - draw_h / 2.0;

        let aa_center_x = active_area.x.mul_add(scale, offset_x);
        let aa_center_y = active_area.y.mul_add(scale, offset_y);

        let half_w = (active_area.w * scale) / 2.0;
        let half_h = (active_area.h * scale) / 2.0;

        let rot_rad = active_area.rotation.to_radians();
        let (sin, cos) = rot_rad.sin_cos();

        let mut points = vec![
            (-half_w, -half_h),
            (half_w, -half_h),
            (half_w, half_h),
            (-half_w, half_h),
        ];

        for (px, py) in &mut points {
            let rx = (*px).mul_add(cos, -(*py * sin));
            let ry = (*px).mul_add(sin, *py * cos);
            *px = rx + aa_center_x;
            *py = ry + aa_center_y;
        }

        let osu_playfield_points = if show_osu_playfield && target_w > 0.0 && target_h > 0.0 {
            let h_pf = active_area.h * (1028.0 / target_h);
            let w_pf = (target_h * (1316.0 / 1080.0) / target_w) * active_area.w;
            let y_offset_mm = active_area.h * (18.0 / target_h);

            let pf_half_w = (w_pf * scale) / 2.0;
            let pf_half_h = (h_pf * scale) / 2.0;
            let pf_offset_y = y_offset_mm * scale;

            let mut pf_points = vec![
                (-pf_half_w, -pf_half_h + pf_offset_y),
                (pf_half_w, -pf_half_h + pf_offset_y),
                (pf_half_w, pf_half_h + pf_offset_y),
                (-pf_half_w, pf_half_h + pf_offset_y),
            ];

            for (px, py) in &mut pf_points {
                let rx = (*px).mul_add(cos, -(*py * sin));
                let ry = (*px).mul_add(sin, *py * cos);
                *px = rx + aa_center_x;
                *py = ry + aa_center_y;
            }
            Some(pf_points)
        } else {
            None
        };

        Self {
            scale,
            offset_x,
            offset_y,
            aa_center_x,
            aa_center_y,
            points,
            osu_playfield_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::models::ActiveArea;

    #[test]
    fn test_active_area_geometry() {
        let active_area = ActiveArea {
            x: 50.0,
            y: 50.0,
            w: 100.0,
            h: 100.0,
            rotation: 0.0,
        };

        let geo = ActiveAreaGeometry::calculate(
            200.0,
            200.0, // phys
            400.0,
            400.0, // viz
            200.0,
            200.0, // viz center
            &active_area,
            1920.0,
            1080.0,
            false,
        );

        // Scale should be (400/200) * 0.8 = 1.6
        assert!((geo.scale - 1.6).abs() < 1e-6);

        // Offset should be 200 - (200 * 1.6 / 2) = 200 - 160 = 40
        assert!((geo.offset_x - 40.0).abs() < 1e-6);
        assert!((geo.offset_y - 40.0).abs() < 1e-6);

        // AA Center should be 40 + 50 * 1.6 = 40 + 80 = 120
        assert!((geo.aa_center_x - 120.0).abs() < 1e-6);
        assert!((geo.aa_center_y - 120.0).abs() < 1e-6);
    }
}
