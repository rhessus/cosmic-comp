// SPDX-License-Identifier: GPL-3.0-only

//! Calcul du contour extérieur (polygone) de l'union de rectangles (écrans).
//! Port de l'algorithme du POC (poc/index.html) : fusion des rects par arêtes partagées,
//! puis parcours du contour.

use smithay::utils::{Logical, Point, Rectangle, Size};

/// Arête d'un rectangle (ligne horizontale ou verticale).
/// index 0 = haut, 1 = droite, 2 = bas, 3 = gauche.
#[derive(Debug, Clone)]
struct Edge {
    index: usize,
    start: i32,
    end: i32,
    pos: i32,
}

fn edge_start(edge: &Edge) -> (i32, i32) {
    if edge.index % 2 == 0 {
        (edge.start, edge.pos)
    } else {
        (edge.pos, edge.start)
    }
}

fn edge_end(edge: &Edge) -> (i32, i32) {
    if edge.index % 2 == 0 {
        (edge.end, edge.pos)
    } else {
        (edge.pos, edge.end)
    }
}

fn points_equal(p1: (i32, i32), p2: (i32, i32)) -> bool {
    p1.0 == p2.0 && p1.1 == p2.1
}

/// Construit les 4 arêtes d'un rectangle (x, y, w, h) en Logical.
fn rect_to_edges(r: &Rectangle<i32, Logical>) -> [Edge; 4] {
    let x = r.loc.x;
    let y = r.loc.y;
    let w = r.size.w;
    let h = r.size.h;
    [
        Edge {
            index: 0,
            start: x,
            end: x + w,
            pos: y,
        },
        Edge {
            index: 1,
            start: y,
            end: y + h,
            pos: x + w,
        },
        Edge {
            index: 2,
            start: x + w,
            end: x,
            pos: y + h,
        },
        Edge {
            index: 3,
            start: y + h,
            end: y,
            pos: x,
        },
    ]
}

/// Fusionne les rectangles qui partagent une arête (même pos, start/end inversés).
/// Modifie `rectangles` en place.
fn merge_rectangles(rectangles: &mut Vec<[Edge; 4]>) {
    let mut index = 0;
    while index < rectangles.len() {
        let rect_idx = index;
        let rectangle = &mut rectangles[rect_idx];
        let mut shared_edge: Option<(usize, usize)> = None; // (edge_index, other_rect_idx)

        for i in 0..rectangles.len() {
            if i == rect_idx {
                continue;
            }
            for (border, edge1) in rectangle.iter().enumerate() {
                let edge2 = &rectangles[i][(border + 2) % 4];
                if edge1.pos == edge2.pos && edge1.start == edge2.end && edge1.end == edge2.start {
                    shared_edge = Some((border, i));
                    break;
                }
            }
            if shared_edge.is_some() {
                break;
            }
        }

        if let Some((edge_index, other_idx)) = shared_edge {
            let rect = rectangles[other_idx].clone();
            let rectangle = &mut rectangles[rect_idx];

            rectangle[edge_index] = rect[edge_index].clone();
            let b1 = (edge_index + 1) % 4;
            rectangle[b1].end = rectangle[b1].end;
            let b2 = (edge_index + 3) % 4;
            rectangle[b2].end = rect[b2].end;

            rectangles.remove(other_idx);
            index = 0;
        } else {
            index += 1;
        }
    }
}

/// Parcourt le contour extérieur à partir du premier rectangle, arête 0.
fn calculate_contour_rec(
    rectangles: &[[Edge; 4]],
    rectangle_index: usize,
    edge_index: usize,
    contour: &mut Vec<(i32, i32)>,
) {
    let edge = &rectangles[rectangle_index][edge_index];
    if contour.is_empty() {
        contour.push(edge_start(edge));
    }

    let mut crossing: Option<(i32, usize, usize)> = None; // (end, rect_idx, edge_idx)
    for (idx, edges) in rectangles.iter().enumerate() {
        if idx == rectangle_index {
            continue;
        }
        let test_edge = &edges[(edge_index + 2) % 4];
        if edge.pos != test_edge.pos {
            continue;
        }
        let overlap = edge.start.max(edge.end) >= test_edge.start.min(test_edge.end)
            && test_edge.start.max(test_edge.end) >= edge.start.min(edge.end);
        if !overlap {
            continue;
        }
        let end = test_edge.end;
        if crossing.map_or(true, |(e, _, _)| end < e) {
            crossing = Some((end, idx, test_edge.index));
        }
    }

    if let Some((cross_end, other_rect_idx, other_edge_index)) = crossing {
        contour.push((
            if edge.index % 2 == 0 {
                cross_end
            } else {
                edge.pos
            },
            if edge.index % 2 == 0 {
                edge.pos
            } else {
                cross_end
            },
        ));
        calculate_contour_rec(
            rectangles,
            other_rect_idx,
            (other_edge_index + 1) % 4,
            contour,
        );
        return;
    }

    let point = edge_end(edge);
    contour.push(point);

    if points_equal(point, contour[0]) {
        return;
    }

    calculate_contour_rec(rectangles, rectangle_index, (edge_index + 1) % 4, contour);
}

/// Calcule le contour extérieur (polygone fermé) de l'union des rectangles.
/// Retourne une liste de points formant le contour, ou `Vec::new()` si aucun rect.
pub fn compute_contour(
    rects: impl IntoIterator<Item = Rectangle<i32, Logical>>,
) -> Vec<Point<i32, Logical>> {
    let rects: Vec<Rectangle<i32, Logical>> = rects.into_iter().collect();
    if rects.is_empty() {
        return vec![];
    }

    let mut rectangles: Vec<[Edge; 4]> = rects.iter().map(rect_to_edges).collect();
    merge_rectangles(&mut rectangles);

    if rectangles.is_empty() {
        return vec![];
    }

    let mut contour = vec![];
    calculate_contour_rec(&rectangles, 0, 0, &mut contour);

    contour
        .into_iter()
        .map(|(x, y)| Point::from((x, y)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::from_loc_and_size(Point::from((x, y)), Size::from((w, h)))
    }

    #[test]
    fn test_contour_empty() {
        let r: Vec<Rectangle<i32, Logical>> = vec![];
        let c = compute_contour(r);
        assert!(c.is_empty());
    }

    #[test]
    fn test_contour_single_rect() {
        let r = vec![rect(0, 0, 100, 50)];
        let c = compute_contour(r);
        assert_eq!(c.len(), 5);
        assert_eq!(c[0], c[4]);
        assert_eq!(c[0], Point::from((0, 0)));
        assert_eq!(c[1], Point::from((100, 0)));
        assert_eq!(c[2], Point::from((100, 50)));
        assert_eq!(c[3], Point::from((0, 50)));
    }

    #[test]
    fn test_contour_poc_outputs() {
        let rects = vec![
            rect(140, 480, 480, 240),
            rect(100, 0, 240, 480),
            rect(140 + 480, 240 + 48, 480, 240),
            rect(140 + 480, 480 + 48, 480, 240),
        ];
        let c = compute_contour(rects);
        assert!(!c.is_empty());
        assert_eq!(c[0], *c.last().unwrap(), "contour must be closed");
    }
}
