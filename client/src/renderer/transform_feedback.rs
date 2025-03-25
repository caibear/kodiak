
use super::{DefaultRender, GpuBuffer, GpuBufferType, Index, InstanceBufferBinding, Renderer, TriangleBuffer, Vertex};
use super::gl::{Gl, Ovao, OvaoCompat};
use web_sys::{WebGlBuffer, WebGlVertexArrayObject, WebGlTransformFeedback};
use std::ops::Range;

struct RecurrentBuffer<R> {
    buffer: GpuBuffer<R, { GpuBufferType::Array.to() }>,
    feedback_vao: WebGlVertexArrayObject,
    feedback: WebGlTransformFeedback,
    instance_vao: WebGlVertexArrayObject,
    last_vertex_buffer: Option<WebGlBuffer>,
}

/// Like a [`InstanceBuffer`] but some of its attributes can be modified by the vertex shader to be used next frame.
pub struct RecurrentInstanceBuffer<S, R> {
    static_buffer: GpuBuffer<S, { GpuBufferType::Array.to() }>,
    recurrent_buffers: [RecurrentBuffer<R>; 2],
}

impl<S: Vertex, R: Vertex> DefaultRender for RecurrentInstanceBuffer<S, R> {
    fn new(renderer: &Renderer) -> Self {
        let gl = &renderer.gl;
        let ovao = &renderer.ovao;

        let static_buffer = GpuBuffer::new(gl);
        let mut recurrent_buffers = std::array::from_fn(|_| {
            let feedback_vao = renderer.ovao.create_vertex_array_oes().unwrap();
            // Make sure VAO was unbound.
            debug_assert!(gl
                .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
                .unwrap()
                .is_null());
            ovao.bind_vertex_array_oes(Some(&feedback_vao));

            let attribs = static_buffer.bind(gl).bind_attribs();

            let buffer = GpuBuffer::new(gl);
            buffer.bind(gl).bind_attribs_with_previous(attribs);

            // Unbinding VAO is ALWAYS required (unlike all other render unbinds).
            ovao.bind_vertex_array_oes(None);

            let feedback = renderer.gl.create_transform_feedback().unwrap();
            gl.bind_transform_feedback(Gl::TRANSFORM_FEEDBACK, Some(&feedback));
            gl.bind_buffer_base(Gl::TRANSFORM_FEEDBACK_BUFFER, 0, Some(buffer._elements()));
            gl.bind_transform_feedback(Gl::TRANSFORM_FEEDBACK, None); // Unbind always required.

            let instance_vao =  renderer.ovao.create_vertex_array_oes().unwrap();
            RecurrentBuffer { buffer, feedback_vao, feedback, instance_vao, last_vertex_buffer: None }
        });
        // Make the buffers write to each other.
        let [a, b] = &mut recurrent_buffers;
        std::mem::swap(&mut a.feedback, &mut b.feedback);

        Self {
            static_buffer,
            recurrent_buffers,
        }
    }
}

impl<S: Vertex, R: Vertex> RecurrentInstanceBuffer<S, R> {
    /// Binds the [`RecurrentInstanceBuffer`] to save transform feedback. Optionally can draw points used internally for transform feedback.
    pub fn bind_feedback<'a>(&'a mut self, renderer: &'a Renderer, draw_points: bool) -> RecurrentInstanceBufferBinding<'a, S, R> {
        RecurrentInstanceBufferBinding::new(&renderer.gl, &renderer.ovao, self, !draw_points)
    }

    /// Binds the [`RecurrentInstanceBuffer`] to draw instances.
    pub fn bind_instances<'a, V: Vertex, I: Index>(&'a mut self, renderer: &'a Renderer, triangle_buffer: &'a TriangleBuffer<V, I>) -> InstanceBufferBinding<'a, V, I> {
        let gl = &renderer.gl;
        let aia = renderer
            .aia
            .as_ref()
            .expect("must enable AngleInstancedArrays");
        let ovao = &renderer.ovao;

        let current = &mut self.recurrent_buffers[0];
        let vertex_buffer = triangle_buffer.vertices._elements();
        if current.last_vertex_buffer.as_ref() != Some(vertex_buffer) {
            current.last_vertex_buffer = Some(vertex_buffer.clone());
            // Make sure VAO was unbound.
            debug_assert!(gl
                .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
                .unwrap()
                .is_null());

            ovao.bind_vertex_array_oes(Some(&current.instance_vao));

            let attribs = triangle_buffer.vertices.bind(gl).bind_attribs();

            // Bind element buffer.
            let element_binding = triangle_buffer.indices.bind(gl);

            let attribs = self.static_buffer.bind(gl).bind_attribs_instanced(aia, attribs);
            current.buffer.bind(gl).bind_attribs_instanced(aia, attribs);

            // Unbinding VAO is ALWAYS required (unlike all other render unbinds).
            ovao.bind_vertex_array_oes(None);

            // Element buffer can't be unbound before VAO unbind.
            drop(element_binding);
        }

        InstanceBufferBinding::new(gl, aia, ovao, triangle_buffer, self.static_buffer.len(), &current.instance_vao)
    }

    /// Copies `static_data` and `recurrent_data` into the [`RecurrentInstanceBuffer`].
    /// `static_data` cannot be changed by the shader.
    /// `recurrent_data` is changed by each execution of the transform feedback shader.
    pub fn buffer(&mut self, renderer: &Renderer, static_data: &[S], recurrent_data: &[R]) {
        self.static_buffer.buffer(&renderer.gl, static_data);
        self.recurrent_buffers[0].buffer.buffer(&renderer.gl, recurrent_data);
        // Fixes "Not enough space in bound transform feedback buffers".
        self.recurrent_buffers[1].buffer.resize_zeroed(&renderer.gl, recurrent_data.len());
    }

    /// For debugging.
    pub fn clear_recurrent(&mut self, renderer: &Renderer) {
        let zeroed: Vec<R> = bytemuck::zeroed_vec(self.static_buffer.len());
        self.recurrent_buffers[0].buffer.buffer(&renderer.gl, &zeroed);
    }

    fn current(&self) -> &RecurrentBuffer<R> {
        &self.recurrent_buffers[0]
    }
}

/// A bound [`RecurrentInstanceBuffer`] that can draw points.
pub struct RecurrentInstanceBufferBinding<'a, S: Vertex, R: Vertex> {
    gl: &'a Gl,
    ovao: &'a Ovao,
    buffer: &'a mut RecurrentInstanceBuffer<S, R>,
    discard_points: bool,
}

impl<'a, S: Vertex, R: Vertex> RecurrentInstanceBufferBinding<'a, S, R> {
    fn new(gl: &'a Gl, ovao: &'a Ovao, buffer: &'a mut RecurrentInstanceBuffer<S, R>, discard_points: bool) -> Self {
        // Make sure transform feedback was unbound.
        debug_assert!(gl
            .get_parameter(Gl::TRANSFORM_FEEDBACK_BINDING)
            .unwrap()
            .is_null());
        // Hack: unbind array buffers to prevent "A transform feedback buffer that would be written to is also bound to a non-transform-feedback target"
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);

        // Begin transform feedback.
        gl.bind_transform_feedback(Gl::TRANSFORM_FEEDBACK, Some(&buffer.current().feedback));
        gl.begin_transform_feedback(Gl::POINTS);

        // Make sure buffer was unbound.
        debug_assert!(gl
            .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        ovao.bind_vertex_array_oes(Some(&buffer.current().feedback_vao));

        if discard_points {
            gl.enable(Gl::RASTERIZER_DISCARD);
        }
        Self { gl, ovao, buffer, discard_points }
    }

    /// Draws points.
    pub fn draw(&self) {
        self.draw_range(0..self.buffer.static_buffer.len());
    }

    /// Draws a specified `range` of points. TODO(pub) does this make sense with transform feedback?
    fn draw_range(&self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        if range.end > self.buffer.static_buffer.len() {
            panic!("out of bounds")
        }
        let count = range.end - range.start;
        self.gl.draw_arrays(
            Gl::POINTS,
            range.start.try_into().unwrap(),
            count.try_into().unwrap(),
        )
    }
}

impl<'a, S: Vertex, R: Vertex> Drop for RecurrentInstanceBufferBinding<'a, S, R> {
    fn drop(&mut self) {
        if self.discard_points {
            self.gl.disable(Gl::RASTERIZER_DISCARD);
        }

        // Unbind ALWAYS required (unlike all other render unbinds).
        self.ovao.bind_vertex_array_oes(None);

        // End transform feedback.
        self.gl.end_transform_feedback();
        self.gl.bind_transform_feedback(Gl::TRANSFORM_FEEDBACK, None); // Unbind always required.

        // Swap output and current instead of copying.
        self.buffer.recurrent_buffers.swap(0, 1);
    }
}
