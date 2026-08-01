// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Tier 2: convert SVG elliptical-arc commands (A) to cubic Béziers.
//! Android VectorDrawable pathData does not support arcs, so each A is
//! approximated by up to 4 cubic segments per the SVG implementation notes.

use std::f64::consts::PI;

/// Returns a sequence of cubic control points (x1,y1,x2,y2,x,y) approximating
/// the arc from (x0,y0) to (x,y) with the given rx,ry,x_axis_rotation(deg),
/// large_arc flag and sweep flag.
pub fn arc_to_cubics(
    x0: f64, y0: f64,
    mut rx: f64, mut ry: f64, x_axis_rot_deg: f64,
    large_arc: bool, sweep: bool,
    x: f64, y: f64,
) -> Vec<[f64; 6]> {
    // Degenerate: zero radius or same point -> straight line as a single cubic.
    if rx == 0.0 || ry == 0.0 || (x0 == x && y0 == y) {
        return vec![[x0, y0, x, y, x, y]];
    }
    rx = rx.abs();
    ry = ry.abs();
    let phi = x_axis_rot_deg.to_radians();
    let (cos_phi, sin_phi) = (phi.cos(), phi.sin());

    // Step 1: compute (x1', y1')
    let dx = (x0 - x) / 2.0;
    let dy = (y0 - y) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    // Correct out-of-range radii
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    // Step 2: compute center (cx', cy')
    let sign = if large_arc != sweep { 1.0 } else { -1.0 };
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let coef = sign * (num.max(0.0) / den).sqrt();
    let cxp = coef * (rx * y1p) / ry;
    let cyp = coef * -(ry * x1p) / rx;

    // Step 3: center in original coords
    let cx = cos_phi * cxp - sin_phi * cyp + (x0 + x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y0 + y) / 2.0;

    // Step 4: start and sweep angles
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| -> f64 {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 { a = -a; }
        a
    };
    let theta1 = ang(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
    if !sweep && dtheta > 0.0 { dtheta -= 2.0 * PI; }
    if sweep && dtheta < 0.0 { dtheta += 2.0 * PI; }

    // Step 5: split into segments <= 90 degrees, approximate each by a cubic.
    let segs = (dtheta.abs() / (PI / 2.0)).ceil().max(1.0) as usize;
    let delta = dtheta / segs as f64;
    let t = (4.0 / 3.0) * (delta / 4.0).tan();

    let mut out = Vec::with_capacity(segs);
    let mut th = theta1;
    let pt = |ang: f64| -> (f64, f64) {
        let (ca, sa) = (ang.cos(), ang.sin());
        (
            cx + rx * ca * cos_phi - ry * sa * sin_phi,
            cy + rx * ca * sin_phi + ry * sa * cos_phi,
        )
    };
    for _ in 0..segs {
        let th2 = th + delta;
        let (x1, y1) = pt(th);
        let (x2, y2) = pt(th2);
        // tangent-based control points
        let (sa1, ca1) = (th.sin(), th.cos());
        let (sa2, ca2) = (th2.sin(), th2.cos());
        let c1x = x1 + t * (-rx * ca1 * 0.0 - rx * sa1 * cos_phi - ry * ca1 * sin_phi);
        // The clean formulation:
        let d1x = -rx * sa1 * cos_phi - ry * ca1 * sin_phi;
        let d1y = -rx * sa1 * sin_phi + ry * ca1 * cos_phi;
        let d2x = -rx * sa2 * cos_phi - ry * ca2 * sin_phi;
        let d2y = -rx * sa2 * sin_phi + ry * ca2 * cos_phi;
        let _ = c1x;
        out.push([
            x1 + t * d1x, y1 + t * d1y,
            x2 - t * d2x, y2 - t * d2y,
            x2, y2,
        ]);
        th = th2;
    }
    out
}
