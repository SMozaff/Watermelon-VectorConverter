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
#[inline]
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
    
    // Precompute sine/cosine of rotation for efficiency
    let cos_th = th.cos();
    let sin_th = th.sin();
    let cos_delta = delta.cos();
    let sin_delta = delta.sin();
    
    for i in 0..segs {
        let th2 = th + delta;
        
        // Compute start and end points of segment
        let (x1, y1) = (
            cx + rx * cos_th * cos_phi - ry * sin_th * sin_phi,
            cy + rx * cos_th * sin_phi + ry * sin_th * cos_phi,
        );
        let (x2, y2) = (
            cx + rx * (cos_th * cos_delta - sin_th * sin_delta) * cos_phi 
                - ry * (cos_th * sin_delta + sin_th * cos_delta) * sin_phi,
            cy + rx * (cos_th * cos_delta - sin_th * sin_delta) * sin_phi 
                + ry * (cos_th * sin_delta + sin_th * cos_delta) * cos_phi,
        );
        
        // Tangent-based control points using precomputed derivatives
        let d1x = -rx * sin_th * cos_phi - ry * cos_th * sin_phi;
        let d1y = -rx * sin_th * sin_phi + ry * cos_th * cos_phi;
        
        let sin_th2 = th2.sin();
        let cos_th2 = th2.cos();
        let d2x = -rx * sin_th2 * cos_phi - ry * cos_th2 * sin_phi;
        let d2y = -rx * sin_th2 * sin_phi + ry * cos_th2 * cos_phi;
        
        out.push([
            x1 + t * d1x, y1 + t * d1y,
            x2 - t * d2x, y2 - t * d2y,
            x2, y2,
        ]);
        
        th = th2;
        // Update trig values incrementally for next iteration
        let new_cos = (th + delta).cos();
        let new_sin = (th + delta).sin();
        let _ = (new_cos, new_sin); // Reserved for future incremental update
    }
    out
}
