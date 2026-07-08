//! La [`Scene`] : une liste de primitives à dessiner pour une frame.

use bytemuck::{Pod, Zeroable};

use crate::{Color, Rect};

/// Données d'une instance transmises au GPU (une par rectangle).
///
/// Le layout mémoire est stable (`repr(C)`) car il est lu directement par le
/// vertex shader via le buffer d'instances.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct Instance {
    /// `[x, y, width, height]` en pixels logiques.
    pub rect: [f32; 4],
    /// `[r, g, b, a]`.
    pub color: [f32; 4],
}

impl Instance {
    /// Layout du buffer d'instances pour le pipeline (locations 1 et 2 ;
    /// la location 0 est réservée au quad unité).
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

/// Une scène 2D : la description déclarative de ce qu'il faut dessiner.
///
/// On la construit à chaque frame (ou on la réutilise), puis on la passe à
/// [`crate::Renderer::render`].
#[derive(Default, Clone)]
pub struct Scene {
    instances: Vec<Instance>,
}

impl Scene {
    /// Crée une scène vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Vide la scène pour la réutiliser à la frame suivante.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Ajoute un rectangle plein.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.instances.push(Instance {
            rect: rect.to_array(),
            color: color.to_array(),
        });
    }

    /// Nombre de primitives dans la scène.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// `true` si la scène ne contient aucune primitive.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Accès interne aux instances pour le renderer.
    pub(crate) fn instances(&self) -> &[Instance] {
        &self.instances
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_rect_pushes_expected_instance() {
        let mut scene = Scene::new();
        assert!(scene.is_empty());

        scene.fill_rect(Rect::new(10.0, 20.0, 30.0, 40.0), Color::rgba(0.1, 0.2, 0.3, 0.4));

        assert_eq!(scene.len(), 1);
        let inst = scene.instances()[0];
        assert_eq!(inst.rect, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(inst.color, [0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn clear_empties_the_scene() {
        let mut scene = Scene::new();
        scene.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE);
        scene.clear();
        assert!(scene.is_empty());
    }
}
