pub use thin_engine::prelude::*;
#[derive(Clone, Copy, Debug)]
pub enum ColliderType { Cylinder, Cube, Sphere }
impl ColliderType {
    pub fn is_inside(self, point: Vec3) -> bool {
        match self {
            Self::Sphere => point.length() <= 1.0,
            Self::Cube => point.x.abs() <= 1.0 && point.y.abs() <= 1.0 && point.z.abs() <= 1.0,
            Self::Cylinder => point.y.abs() <= 1.0 && vec2(point.x, point.z).length() <= 1.0,
        }
    }
    pub fn is_inside_transformed(self, point: Vec3, trans: Mat4) -> bool {
        self.is_inside(point.extend(1.0).transform(&trans.inverse()).truncate())
    }
    pub fn move_out(&self, point: Vec3) -> Option<Vec3> {
        if !self.is_inside(point) { return None }
        Some(self.onto_surface(point) - point)
    }
    pub fn move_out_transformed(self, point: Vec3, trans: Mat4) -> Option<Vec3> {
        Some(
            self.move_out(point.extend(1.0).transform(&trans.inverse()).truncate())?
                .transform(&trans.into())
        )
    }
    pub fn onto_surface(&self, point: Vec3) -> Vec3 {
        match self {
            Self::Sphere => point.normalise(),
            Self::Cube => {
                let mut result = vec3(
                    point.x.clamp(-1.0 ,1.0),
                    point.y.clamp(-1.0, 1.0),
                    point.z.clamp(-1.0, 1.0)
                );
                let dif_x = 1.0 - result.x.abs();
                let dif_y = 1.0 - result.y.abs();
                let dif_z = 1.0 - result.z.abs();
                
                if      dif_x < dif_y && dif_x < dif_z { result.x = result.x.signum() }
                else if dif_y < dif_x && dif_y < dif_z { result.y = result.y.signum() }
                else if dif_z < dif_x && dif_z < dif_y { result.z = result.z.signum() }
                result
            },
            Self::Cylinder => {
                let circle_dif = 1.0 - vec2(point.x, point.z).length().min(1.0);
                let circle = vec2(point.x, point.z).normalise();

                if circle_dif < 1.0 - point.y.abs().min(1.0) { vec3(circle.x, point.y.clamp(-1.0, 1.0), circle.y) }
                else {
                    let min_x = circle.x.abs();
                    let min_z = circle.y.abs();
                    vec3(point.x.clamp(-min_x, min_x), point.y.signum(), point.z.clamp(-min_z, min_z))
                }
            },
        } 
    }
    pub fn onto_surface_transformed(self, point: Vec3, trans: Mat4) -> Vec3 {
        self.onto_surface(
            point.extend(1.0).transform(&trans.inverse()).truncate()
        ).extend(1.0).transform(&trans).truncate()
    }
    pub fn move_out_of(self, trans: Mat4, other: Self, other_trans: Mat4) -> Option<Vec3> {
        let self_centre  = vec4(0.0, 0.0, 0.0, 1.0).transform(&trans)      .truncate();
        let other_centre = vec4(0.0, 0.0, 0.0, 1.0).transform(&other_trans).truncate();

        let other_onto = self.onto_surface_transformed(other_centre, trans);
        let self_onto = other.onto_surface_transformed(self_centre, other_trans);
        
        if other_onto.distance(other_centre) < self_onto.distance(self_centre) {
            Some(other.move_out_transformed(other_onto, other_trans)?.transform(&trans.into()))
        } else {
            Some(-self.move_out_transformed(self_onto,  trans)?.transform(&other_trans.into()))
        }
    }
    pub fn is_inside_of(self, trans: Mat4, other: Self, other_trans: Mat4) -> bool {
        let self_centre  = vec4(0.0, 0.0, 0.0, 1.0).transform(&trans)      .truncate();
        let other_centre = vec4(0.0, 0.0, 0.0, 1.0).transform(&other_trans).truncate();

        let other_onto = self.onto_surface_transformed(other_centre, trans);
        let self_onto = other.onto_surface_transformed(self_centre, other_trans);
        
        if other_onto.distance(other_centre) < self_onto.distance(self_centre) {
            other.is_inside_transformed(other_onto, other_trans)
        } else {
            self.is_inside_transformed(self_onto, trans)
        }
    }
}



#[test]
fn on_surface() {
    assert_eq!(
        ColliderType::Cube.onto_surface(vec3(0.3, 0.4, -1.5)),
        vec3(0.3, 0.4, -1.0)
    );
    assert_eq!(
        ColliderType::Cube.onto_surface(vec3(-1.1, 1.4, 1.5)),
        vec3(-1.0, 1.0, 1.0)
    );
}
