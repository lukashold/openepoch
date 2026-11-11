//! Native CHOP53 endpoint proposals for a FootCitizen's ordinary City Build child.

const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
const TAU: f64 = f32::from_bits(0x40c9_0fdb) as f64;
const HALF_X: f64 = f32::from_bits(0x401e_6455) as f64;
const HALF_Y: f64 = f32::from_bits(0x4007_c3b6) as f64;
// 8736E0 and 870740 add the .125 FootCitizen extent to the requested .3.
// The .135 lower bound leaves no room for 873800's optional .5 shrink.
const MAX_MARGIN: f64 = f32::from_bits(0x3ed9_999a) as f64;
// 873A00 converts ((HALF_X + MAX_MARGIN) / .3125) * PI + .5 to 29,
// then subtracts one. This ordinary 7F13B0 call does not cap the count.
const HALF_TURN_STEPS: u32 = 28;

pub(crate) trait BuildWaypointAdmission {
    fn ground_height(&mut self, xy: [f32; 2]) -> Option<f32>;
    /// The original point gate ignores the worker for squared XYZ distance < 225.
    fn point(&mut self, worker: [f32; 3], candidate: [f32; 3]) -> bool;
    fn inward(&mut self, from: [f32; 3], site: [f32; 3]) -> bool;
    fn outward(&mut self, from: [f32; 3], to: [f32; 2]) -> bool;
}

fn chop(value: f64) -> f32 {
    let nearest = value as f32;
    if f64::from(nearest).abs() > value.abs() {
        f32::from_bits(nearest.to_bits() - 1)
    } else {
        nearest
    }
}

fn offset(from: [f32; 3], to: [f32; 3]) -> [f32; 2] {
    [0, 1].map(|axis| chop(f64::from(to[axis]) - f64::from(from[axis])))
}

fn sin_cos(angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [cos, sin].map(|value| f64::from(chop(value)))
}

fn rotate([x, y]: [f64; 2], [cos, sin]: [f64; 2]) -> [f64; 2] {
    [x * cos - y * sin, y * cos + x * sin].map(|value| f64::from(chop(value)))
}

// 8740A0 spills the normalized X before applying the distance. Distance
// is one in this Build branch. Y stays in the register until world addition.
fn outward_endpoint(from: [f32; 3], site: [f32; 3]) -> Option<[f32; 2]> {
    let [x, y] = offset(site, from).map(f64::from);
    let length = (x * x + y * y).sqrt();
    if length == 0.0 {
        return None;
    }
    let inverse = length.recip();
    Some([
        chop(f64::from(from[0]) + f64::from(chop(x * inverse))),
        chop(f64::from(from[1]) + y * inverse),
    ])
}

fn projection(site: [f32; 3], ray: [f64; 2], orientation: [f64; 2]) -> [f32; 2] {
    let [x, y] = ray;
    let use_y = x == 0.0 || f64::from(chop(y / x)).abs() > HALF_Y / HALF_X;
    let (half, axis) = if use_y { (HALF_Y, y) } else { (HALF_X, x) };
    let scale = (half + MAX_MARGIN) / axis.abs();
    let [x, y] = rotate(ray, orientation);
    // 873280 spills the projected Y before adding the site Y. X stays wide.
    [
        chop(f64::from(site[0]) + x * scale),
        chop(f64::from(site[1]) + f64::from(chop(y * scale))),
    ]
}

/// The direct geometry bit comes from 7FDF80's rectangle check. Direct
/// checks run inward then outward, while radial candidates run sampled
/// height, point, outward, then inward. The caller owns both movement gates
/// and the later worker-to-endpoint route search.
pub(crate) fn search(
    worker: [f32; 3],
    site: [f32; 3],
    site_yaw: f32,
    direct_geometry: bool,
    admission: &mut impl BuildWaypointAdmission,
) -> Option<[f32; 3]> {
    if !worker.into_iter().chain(site).all(f32::is_finite)
        || !site_yaw.is_finite()
        || f64::from(site_yaw).abs() > TAU
    {
        return None;
    }
    if direct_geometry
        && admission.inward(worker, site)
        && outward_endpoint(worker, site).is_some_and(|to| admission.outward(worker, to))
    {
        return Some(worker);
    }

    let [x, y] = offset(site, worker).map(f64::from);
    let angle = y.atan2(x);
    let angle = f64::from(chop(if angle < 0.0 { angle + TAU } else { angle }));
    let yaw = f64::from(site_yaw);
    let relative = angle - yaw;
    let mut positive = sin_cos(if relative < 0.0 {
        relative + TAU
    } else {
        relative
    });
    let mut negative = positive;
    let orientation = sin_cos(yaw);
    let [cos_step, sin_step] = sin_cos(PI / f64::from(HALF_TURN_STEPS));

    for index in 0..=HALF_TURN_STEPS {
        let count = if index == 0 || index == HALF_TURN_STEPS {
            1
        } else {
            2
        };
        for ray in [positive, negative].into_iter().take(count) {
            let xy = projection(site, ray, orientation);
            let Some(z) = admission.ground_height(xy).filter(|z| z.is_finite()) else {
                continue;
            };
            let candidate = [xy[0], xy[1], z];
            if admission.point(worker, candidate)
                && outward_endpoint(candidate, site)
                    .is_some_and(|to| admission.outward(candidate, to))
                && admission.inward(candidate, site)
            {
                return Some(candidate);
            }
        }
        positive = rotate(positive, [cos_step, sin_step]);
        negative = rotate(negative, [cos_step, -sin_step]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const Z: f32 = f32::from_bits(0x4041_5ec6);
    const SITE: [f32; 3] = [72.0, 24.0, Z];
    const NEAR: [f32; 3] = [f32::from_bits(0x4295_69a4), f32::from_bits(0x41c3_076f), Z];
    const FAR: [f32; 3] = [f32::from_bits(0x429f_c0b6), f32::from_bits(0x41b3_0397), Z];

    #[derive(Debug, PartialEq)]
    enum Probe {
        Height([u32; 2]),
        Point([u32; 3]),
        Inward([u32; 3]),
        Outward([u32; 2]),
    }

    #[derive(Default)]
    struct Fixture {
        probes: Vec<Probe>,
        denied: Option<usize>,
        denied_points: usize,
        denied_outward: usize,
    }

    impl Fixture {
        fn push(&mut self, probe: Probe) -> bool {
            self.probes.push(probe);
            self.denied != Some(self.probes.len() - 1)
        }
    }

    impl BuildWaypointAdmission for Fixture {
        fn ground_height(&mut self, xy: [f32; 2]) -> Option<f32> {
            self.push(Probe::Height(xy.map(f32::to_bits))).then_some(Z)
        }

        fn point(&mut self, _: [f32; 3], candidate: [f32; 3]) -> bool {
            let allowed =
                self.push(Probe::Point(candidate.map(f32::to_bits))) && self.denied_points == 0;
            self.denied_points = self.denied_points.saturating_sub(1);
            allowed
        }

        fn inward(&mut self, from: [f32; 3], site: [f32; 3]) -> bool {
            assert_eq!(site, SITE);
            self.push(Probe::Inward(from.map(f32::to_bits)))
        }

        fn outward(&mut self, _: [f32; 3], to: [f32; 2]) -> bool {
            let allowed =
                self.push(Probe::Outward(to.map(f32::to_bits))) && self.denied_outward == 0;
            self.denied_outward = self.denied_outward.saturating_sub(1);
            allowed
        }
    }

    #[test]
    fn measured_chop_endpoints_and_gate_order() {
        // Approach273 raw CHOP cases 0..3, identical in both installed images.
        let mut clear = Fixture::default();
        assert_eq!(search(NEAR, SITE, 0.0, true, &mut clear), Some(NEAR));
        let inward = Probe::Inward(NEAR.map(f32::to_bits));
        let outward = Probe::Outward([0x4297_64b3, 0x41c4_2332]);
        assert_eq!(clear.probes, [inward, outward]);

        let mut far = Fixture::default();
        let far_point = [0x4295_ccbc, 0x41bb_380b, Z.to_bits()];
        assert_eq!(
            search(FAR, SITE, 0.0, false, &mut far).map(|p| p.map(f32::to_bits)),
            Some(far_point)
        );
        assert_eq!(
            far.probes,
            [
                Probe::Height([far_point[0], far_point[1]]),
                Probe::Point(far_point),
                Probe::Outward([0x4297_c232, 0x41b9_9aa8]),
                Probe::Inward(far_point)
            ]
        );

        for denied in [0, 1] {
            let mut blocked = Fixture {
                denied: Some(denied),
                ..Fixture::default()
            };
            let near_point = [0x4295_ccbc, 0x41c3_3ee3, Z.to_bits()];
            assert_eq!(
                search(NEAR, SITE, 0.0, true, &mut blocked).map(|p| p.map(f32::to_bits)),
                Some(near_point)
            );
            assert_eq!(blocked.probes[0], Probe::Inward(NEAR.map(f32::to_bits)));
            if denied == 1 {
                assert_eq!(
                    blocked.probes[1],
                    Probe::Outward([0x4297_64b3, 0x41c4_2332])
                );
            }
            assert_eq!(
                blocked.probes[denied + 1..],
                [
                    Probe::Height([near_point[0], near_point[1]]),
                    Probe::Point(near_point),
                    Probe::Outward([0x4297_c7cb, 0x41c4_5aa5]),
                    Probe::Inward(near_point)
                ]
            );
        }
    }

    #[test]
    fn rejected_points_advance_in_native_angular_order() {
        // Segments274 CHOP cases 7 and 8 reject five points before admission.
        for (worker, direct, y_words) in [
            (
                NEAR,
                true,
                [
                    0x41c3_3ee3,
                    0x41c5_f412,
                    0x41c0_9f38,
                    0x41c8_d288,
                    0x41be_0399,
                    0x41cb_f2c1,
                ],
            ),
            (
                FAR,
                false,
                [
                    0x41bb_380b,
                    0x41bd_e1ca,
                    0x41b8_6de1,
                    0x41c0_7da8,
                    0x41b5_6d31,
                    0x41c3_1cb6,
                ],
            ),
        ] {
            let mut fixture = Fixture {
                denied_points: 5,
                denied_outward: usize::from(direct),
                ..Fixture::default()
            };
            let expected = y_words.map(|y| [0x4295_ccbc, y, Z.to_bits()]);
            assert_eq!(
                search(worker, SITE, 0.0, direct, &mut fixture).map(|p| p.map(f32::to_bits)),
                Some(expected[5])
            );
            let point_order = fixture
                .probes
                .iter()
                .filter_map(|probe| match probe {
                    Probe::Point(point) => Some(*point),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(point_order, expected);
            assert_eq!(
                fixture.probes.len(),
                14 + usize::from(direct) * 2,
                "Rejected points skip both segment gates"
            );
        }

        let mut exhausted = Fixture {
            denied_points: usize::MAX,
            ..Fixture::default()
        };
        assert_eq!(search(FAR, SITE, 0.0, false, &mut exhausted), None);
        assert_eq!(
            exhausted.probes.len(),
            112,
            "56 points including both single antipodes"
        );
    }

    #[test]
    fn blocked_outward_ray_and_rotated_site_match_native() {
        // Segments274 original CHOP case 3. One blocked leaf rejects the
        // direct outward ray and the first radial outward ray.
        let mut blocked = Fixture {
            denied_outward: 2,
            ..Fixture::default()
        };
        let first = [0x4295_ccbc, 0x41c3_3ee3, Z.to_bits()];
        let second = [0x4295_ccbc, 0x41c5_f412, Z.to_bits()];
        assert_eq!(
            search(NEAR, SITE, 0.0, true, &mut blocked).map(|p| p.map(f32::to_bits)),
            Some(second)
        );
        assert_eq!(
            blocked.probes,
            [
                Probe::Inward(NEAR.map(f32::to_bits)),
                Probe::Outward([0x4297_64b3, 0x41c4_2332]),
                Probe::Height([first[0], first[1]]),
                Probe::Point(first),
                Probe::Outward([0x4297_c7cb, 0x41c4_5aa5]),
                Probe::Height([second[0], second[1]]),
                Probe::Point(second),
                Probe::Outward([0x4297_bcaa, 0x41c7_f123]),
                Probe::Inward(second),
            ]
        );

        // Case 6 rotates the original rectangle +45 degrees.
        let mut rotated = Fixture::default();
        let point = [0x4297_3206, 0x41c4_06d6, Z.to_bits()];
        assert_eq!(
            search(NEAR, SITE, f32::from_bits(0x3f49_0fdb), false, &mut rotated)
                .map(|p| p.map(f32::to_bits)),
            Some(point)
        );
        assert_eq!(
            rotated.probes,
            [
                Probe::Height([point[0], point[1]]),
                Probe::Point(point),
                Probe::Outward([0x4299_2d15, 0x41c5_2299]),
                Probe::Inward(point)
            ]
        );
    }
}
