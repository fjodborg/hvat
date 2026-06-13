//! Contour extraction from binary masks using marching squares algorithm.
//!
//! This module provides functions to extract polygon contours from binary masks,
//! which is needed to convert SAM2 masks into annotation polygons.

use super::SAM2Mask;

/// Extracts the outer contour of a binary mask using marching squares.
///
/// The returned contour is a list of (x, y) coordinates in image space.
/// The contour is simplified using the Douglas-Peucker algorithm to reduce
/// the number of vertices.
///
/// # Arguments
///
/// * `mask` - The binary mask to extract contour from
/// * `epsilon` - Simplification tolerance (higher = fewer points). Typically 1.0-3.0.
///
/// # Returns
///
/// A vector of (x, y) coordinates forming the contour polygon.
pub fn extract_contour(mask: &SAM2Mask, epsilon: f32) -> Vec<(f32, f32)> {
    // Find a starting point on the contour (first foreground pixel with background neighbor)
    let start = find_contour_start(mask);
    let Some((start_x, start_y)) = start else {
        return Vec::new();
    };

    // Trace the contour using marching squares
    let contour = trace_contour(mask, start_x, start_y);

    // Simplify the contour using Douglas-Peucker algorithm
    if contour.len() > 3 {
        douglas_peucker(&contour, epsilon)
    } else {
        contour
    }
}

/// Finds the starting point for contour tracing.
///
/// Returns the first foreground pixel that has at least one background neighbor.
fn find_contour_start(mask: &SAM2Mask) -> Option<(u32, u32)> {
    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.get(x, y) > 127 && is_edge_pixel(mask, x, y) {
                return Some((x, y));
            }
        }
    }
    None
}

/// Checks if a foreground pixel is on the edge (has at least one background neighbor).
fn is_edge_pixel(mask: &SAM2Mask, x: u32, y: u32) -> bool {
    let neighbors = [
        (x.wrapping_sub(1), y),
        (x + 1, y),
        (x, y.wrapping_sub(1)),
        (x, y + 1),
    ];

    for (nx, ny) in neighbors {
        if nx >= mask.width || ny >= mask.height || mask.get(nx, ny) <= 127 {
            return true;
        }
    }
    false
}

/// Traces the contour starting from the given point using marching squares.
///
/// This implements a simplified version of the marching squares algorithm
/// that follows the edge of the binary mask.
fn trace_contour(mask: &SAM2Mask, start_x: u32, start_y: u32) -> Vec<(f32, f32)> {
    let mut contour = Vec::new();
    let mut x = start_x;
    let mut y = start_y;

    // Direction: 0=right, 1=down, 2=left, 3=up
    let mut dir = 0u8;

    // Track visited edges to avoid infinite loops
    let mut visited = std::collections::HashSet::new();

    loop {
        // Add current point to contour (offset by 0.5 to center in pixel)
        contour.push((x as f32 + 0.5, y as f32 + 0.5));

        // Mark this edge as visited
        let edge_key = (x, y, dir);
        if visited.contains(&edge_key) {
            // We've completed the loop
            break;
        }
        visited.insert(edge_key);

        // Find next direction by checking neighbors
        // Priority: turn left, go straight, turn right, go back
        let (dx, dy) = dir_to_delta(dir);
        let (left_dir, left_dx, left_dy) = turn_left(dir);
        let (right_dir, right_dx, right_dy) = turn_right(dir);

        // Check left
        let left_x = (x as i32 + left_dx) as u32;
        let left_y = (y as i32 + left_dy) as u32;
        if left_x < mask.width && left_y < mask.height && mask.get(left_x, left_y) > 127 {
            x = left_x;
            y = left_y;
            dir = left_dir;
            continue;
        }

        // Check straight
        let straight_x = (x as i32 + dx) as u32;
        let straight_y = (y as i32 + dy) as u32;
        if straight_x < mask.width
            && straight_y < mask.height
            && mask.get(straight_x, straight_y) > 127
        {
            x = straight_x;
            y = straight_y;
            continue;
        }

        // Check right
        let right_x = (x as i32 + right_dx) as u32;
        let right_y = (y as i32 + right_dy) as u32;
        if right_x < mask.width && right_y < mask.height && mask.get(right_x, right_y) > 127 {
            x = right_x;
            y = right_y;
            dir = right_dir;
            continue;
        }

        // Go back (180 degree turn)
        dir = (dir + 2) % 4;

        // Check if we're back at start
        if x == start_x && y == start_y {
            break;
        }

        // Safety limit to prevent infinite loops
        if contour.len() > (mask.width * mask.height) as usize {
            log::warn!("Contour tracing exceeded safety limit");
            break;
        }
    }

    contour
}

/// Converts direction to delta (dx, dy).
fn dir_to_delta(dir: u8) -> (i32, i32) {
    match dir {
        0 => (1, 0),  // right
        1 => (0, 1),  // down
        2 => (-1, 0), // left
        3 => (0, -1), // up
        _ => (0, 0),
    }
}

/// Returns the left turn direction and delta.
fn turn_left(dir: u8) -> (u8, i32, i32) {
    let new_dir = (dir + 3) % 4; // -1 mod 4
    let (dx, dy) = dir_to_delta(new_dir);
    (new_dir, dx, dy)
}

/// Returns the right turn direction and delta.
fn turn_right(dir: u8) -> (u8, i32, i32) {
    let new_dir = (dir + 1) % 4;
    let (dx, dy) = dir_to_delta(new_dir);
    (new_dir, dx, dy)
}

/// Simplifies a polyline using the Douglas-Peucker algorithm.
///
/// This reduces the number of points while preserving the shape.
///
/// # Arguments
///
/// * `points` - The input polyline
/// * `epsilon` - Maximum distance tolerance for simplification
///
/// # Returns
///
/// A simplified polyline with fewer points.
fn douglas_peucker(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    if points.len() < 3 {
        return points.to_vec();
    }

    // Find the point with maximum distance from the line segment
    let mut max_dist = 0.0f32;
    let mut max_idx = 0;

    let start = points[0];
    let end = points[points.len() - 1];

    for (i, point) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = point_to_line_distance(*point, start, end);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    // If max distance exceeds epsilon, recursively simplify
    if max_dist > epsilon {
        let left = douglas_peucker(&points[..=max_idx], epsilon);
        let right = douglas_peucker(&points[max_idx..], epsilon);

        // Combine results (excluding duplicate point at max_idx)
        let mut result = left;
        result.extend_from_slice(&right[1..]);
        result
    } else {
        // Keep only endpoints
        vec![start, end]
    }
}

/// Calculates perpendicular distance from a point to a line segment.
fn point_to_line_distance(point: (f32, f32), line_start: (f32, f32), line_end: (f32, f32)) -> f32 {
    let (px, py) = point;
    let (x1, y1) = line_start;
    let (x2, y2) = line_end;

    let dx = x2 - x1;
    let dy = y2 - y1;

    let line_len_sq = dx * dx + dy * dy;

    if line_len_sq < f32::EPSILON {
        // Line segment is a point
        let dpx = px - x1;
        let dpy = py - y1;
        return (dpx * dpx + dpy * dpy).sqrt();
    }

    // Calculate the projection of point onto the line
    let t = ((px - x1) * dx + (py - y1) * dy) / line_len_sq;
    let t = t.clamp(0.0, 1.0);

    // Find the closest point on the line segment
    let closest_x = x1 + t * dx;
    let closest_y = y1 + t * dy;

    // Calculate distance
    let dpx = px - closest_x;
    let dpy = py - closest_y;
    (dpx * dpx + dpy * dpy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_contour_empty_mask() {
        let mask = SAM2Mask::new(vec![0u8; 100], 10, 10, 0.9);
        let contour = extract_contour(&mask, 1.0);
        assert!(contour.is_empty());
    }

    #[test]
    fn test_extract_contour_square() {
        // Create a 10x10 mask with a 4x4 square in the middle
        let mut data = vec![0u8; 100];
        for y in 3..7 {
            for x in 3..7 {
                data[y * 10 + x] = 255;
            }
        }

        let mask = SAM2Mask::new(data, 10, 10, 0.9);
        let contour = extract_contour(&mask, 0.5);

        // Should have at least 4 vertices for a square (corners)
        assert!(contour.len() >= 4);

        // All points should be within the mask bounds
        for (x, y) in &contour {
            assert!(*x >= 0.0 && *x <= 10.0);
            assert!(*y >= 0.0 && *y <= 10.0);
        }
    }

    #[test]
    fn test_douglas_peucker() {
        let points = vec![(0.0, 0.0), (1.0, 0.1), (2.0, 0.0), (3.0, 0.1), (4.0, 0.0)];

        let simplified = douglas_peucker(&points, 0.5);

        // With epsilon 0.5, the middle points should be removed
        assert_eq!(simplified.len(), 2);
        assert_eq!(simplified[0], (0.0, 0.0));
        assert_eq!(simplified[1], (4.0, 0.0));
    }

    #[test]
    fn test_point_to_line_distance() {
        // Point directly above middle of line
        let dist = point_to_line_distance((5.0, 5.0), (0.0, 0.0), (10.0, 0.0));
        assert!((dist - 5.0).abs() < 0.001);

        // Point at endpoint
        let dist = point_to_line_distance((0.0, 0.0), (0.0, 0.0), (10.0, 0.0));
        assert!(dist < 0.001);
    }
}
