use std::num::Float;
use std::num::ToPrimitive;
use std::num::FromPrimitive;
use color::Color;
use dimensions::Dimensions;
use graphics;
use graphics::Graphics;
use graphics::character::CharacterCache;
use label;
use label::FontSize;
use mouse::Mouse;
use point::Point;
use rectangle;
use rectangle::{
    Corner
};
use ui_context::{
    Id,
    UIID,
    UiContext,
};
use utils::{
    clamp,
    map_range,
    val_to_string,
};
use vecmath::{
    vec2_add,
    vec2_sub,
};
use widget::{ DefaultWidgetState, Widget };
use Callback;
use FrameColor;
use FrameWidth;
use LabelText;
use LabelColor;
use LabelFontSize;
use Position;
use Size;

/// Represents the state of the xy_pad widget.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    Normal,
    Highlighted,
    Clicked,
}

impl State {
    /// Return the associated Rectangle state.
    fn as_rectangle_state(&self) -> rectangle::State {
        match self {
            &State::Normal => rectangle::State::Normal,
            &State::Highlighted => rectangle::State::Highlighted,
            &State::Clicked => rectangle::State::Clicked,
        }
    }
}

widget_fns!(XYPad, State, Widget::XYPad(State::Normal));

/// Check the current state of the button.
fn get_new_state(is_over: bool,
                 prev: State,
                 mouse: Mouse) -> State {
    use mouse::ButtonState::{Down, Up};
    use self::State::{Normal, Highlighted, Clicked};
    match (is_over, prev, mouse.left) {
        (true,  Normal,  Down) => Normal,
        (true,  _,       Down) => Clicked,
        (true,  _,       Up)   => Highlighted,
        (false, Clicked, Down) => Clicked,
        _                      => Normal,
    }
}

/// Draw the crosshair.
fn draw_crosshair<B: Graphics>(
    win_w: f64,
    win_h: f64,
    graphics: &mut B,
    pos: Point,
    line_width: f64,
    vert_x: f64, hori_y: f64,
    pad_dim: Dimensions,
    color: Color
) {
    let draw_state = graphics::default_draw_state();
    let transform = graphics::abs_transform(win_w, win_h);
    let Color(col) = color;
    let line = graphics::Line::new(col, 0.5 * line_width);
    line.draw(
        [vert_x, pos[1], vert_x, pos[1] + pad_dim[1]],
        draw_state,
        transform,
        graphics
    );
    line.draw(
        [pos[0], hori_y, pos[0] + pad_dim[0], hori_y],
        draw_state,
        transform,
        graphics
    );
}


/// A context on which the builder pattern can be implemented.
pub struct XYPad<'a, X, Y, F> {
    ui_id: UIID,
    x: X, min_x: X, max_x: X,
    y: Y, min_y: Y, max_y: Y,
    line_width: f64,
    font_size: FontSize,
    pos: Point,
    dim: Dimensions,
    maybe_callback: Option<F>,
    maybe_color: Option<Color>,
    maybe_frame: Option<f64>,
    maybe_frame_color: Option<Color>,
    maybe_label: Option<&'a str>,
    maybe_label_color: Option<Color>,
    maybe_label_font_size: Option<u32>,
}

impl <'a, X, Y, F> XYPad<'a, X, Y, F> {
    #[inline]
    pub fn line_width(self, width: f64) -> XYPad<'a, X, Y, F> {
        XYPad { line_width: width, ..self }
    }
    #[inline]
    pub fn value_font_size(self, size: FontSize) -> XYPad<'a, X, Y, F> {
        XYPad { font_size: size, ..self }
    }
}

impl<'a, X, Y, F> XYPad<'a, X, Y, F> {
    /// An xy_pad builder method to be implemented by the UiContext.
    pub fn new(ui_id: UIID,
              x_val: X, min_x: X, max_x: X,
              y_val: Y, min_y: Y, max_y: Y) -> XYPad<'a, X, Y, F> {
        XYPad {
            ui_id: ui_id,
            x: x_val, min_x: min_x, max_x: max_x,
            y: y_val, min_y: min_y, max_y: max_y,
            line_width: 1.0,
            font_size: 18u32,
            pos: [0.0, 0.0],
            dim: [128.0, 128.0],
            maybe_callback: None,
            maybe_color: None,
            maybe_frame: None,
            maybe_frame_color: None,
            maybe_label: None,
            maybe_label_color: None,
            maybe_label_font_size: None,
        }
    }
}

quack! {
    xy_pad: XYPad['a, X, Y, F]
    get:
        fn () -> Size [] { Size(xy_pad.dim) }
        fn () -> DefaultWidgetState [] {
            DefaultWidgetState(Widget::XYPad(State::Normal))
        }
        fn () -> Id [] { Id(xy_pad.ui_id) }
    set:
        fn (val: Color) [] { xy_pad.maybe_color = Some(val) }
        fn (val: Callback<F>) [where F: FnMut(X, Y) + 'a] {
            xy_pad.maybe_callback = Some(val.0)
        }
        fn (val: FrameColor) [] { xy_pad.maybe_frame_color = Some(val.0) }
        fn (val: FrameWidth) [] { xy_pad.maybe_frame = Some(val.0) }
        fn (val: LabelText<'a>) [] { xy_pad.maybe_label = Some(val.0) }
        fn (val: LabelColor) [] { xy_pad.maybe_label_color = Some(val.0) }
        fn (val: LabelFontSize) [] { xy_pad.maybe_label_font_size = Some(val.0) }
        fn (val: Position) [] { xy_pad.pos = val.0 }
        fn (val: Size) [] { xy_pad.dim = val.0 }
    action:
}

impl<'a, X, Y, F> ::draw::Drawable for XYPad<'a, X, Y, F>
    where
        X: Float + ToPrimitive + FromPrimitive + ToString,
        Y: Float + ToPrimitive + FromPrimitive + ToString,
        F: FnMut(X, Y) + 'a
{

    fn draw<B, C>(&mut self, uic: &mut UiContext<C>, graphics: &mut B)
        where
            B: Graphics<Texture = <C as CharacterCache>::Texture>,
            C: CharacterCache
    {

        // Init.
        let state = *get_state(uic, self.ui_id);
        let mouse = uic.get_mouse_state();
        let frame_w = self.maybe_frame.unwrap_or(uic.theme.frame_width);
        let frame_w2 = frame_w * 2.0;
        let maybe_frame = match frame_w > 0.0 {
            true => Some((frame_w, self.maybe_frame_color.unwrap_or(uic.theme.frame_color))),
            false => None,
        };
        let pad_dim = vec2_sub(self.dim, [frame_w2; 2]);
        let pad_pos = vec2_add(self.pos, [frame_w, frame_w]);
        let is_over_pad = rectangle::is_over(pad_pos, mouse.pos, pad_dim);
        let new_state = get_new_state(is_over_pad, state, mouse);

        // Determine new values.
        let (new_x, new_y) = match (is_over_pad, new_state) {
            (_, State::Normal) | (_, State::Highlighted) => (self.x, self.y),
            (_, State::Clicked) => {
                let temp_x = clamp(mouse.pos[0], pad_pos[0], pad_pos[0] + pad_dim[0]);
                let temp_y = clamp(mouse.pos[1], pad_pos[1], pad_pos[1] + pad_dim[1]);
                (map_range(temp_x - self.pos[0], pad_dim[0], 0.0, self.min_x, self.max_x),
                 map_range(temp_y - self.pos[1], pad_dim[1], 0.0, self.min_y, self.max_y))
            }
        };

        // Callback if value is changed or the pad is clicked/released.
        match self.maybe_callback {
            Some(ref mut callback) => {
                if self.x != new_x || self.y != new_y { (*callback)(new_x, new_y) }
                else {
                    match (state, new_state) {
                        (State::Highlighted, State::Clicked)
                        | (State::Clicked, State::Highlighted) => (*callback)(new_x, new_y),
                        _ => (),
                    }
                }
            },
            None => (),
        }

        // Draw.
        let rect_state = new_state.as_rectangle_state();
        let color = self.maybe_color.unwrap_or(uic.theme.shape_color);
        rectangle::draw(uic.win_w, uic.win_h, graphics, rect_state, self.pos,
                        self.dim, maybe_frame, color);
        let (vert_x, hori_y) = match (is_over_pad, new_state) {
            (_, State::Normal) | (_, State::Highlighted) =>
                (pad_pos[0] + map_range(new_x, self.min_x, self.max_x, pad_dim[0], 0.0),
                 pad_pos[1] + map_range(new_y, self.min_y, self.max_y, pad_dim[1], 0.0)),
            (_, State::Clicked) =>
                (clamp(mouse.pos[0], pad_pos[0], pad_pos[0] + pad_dim[0]),
                 clamp(mouse.pos[1], pad_pos[1], pad_pos[1] + pad_dim[1])),
        };
        // Crosshair.
        draw_crosshair(uic.win_w, uic.win_h, graphics, pad_pos, self.line_width,
                       vert_x, hori_y, pad_dim, color.plain_contrast());
        // Label.
        if let Some(l_text) = self.maybe_label {
            let l_color = self.maybe_label_color.unwrap_or(uic.theme.label_color);
            let l_size = self.maybe_label_font_size.unwrap_or(uic.theme.font_size_medium);
            let l_w = label::width(uic, l_size, l_text);
            let l_x = pad_pos[0] + (pad_dim[0] - l_w) / 2.0;
            let l_y = pad_pos[1] + (pad_dim[1] - l_size as f64) / 2.0;
            let l_pos = [l_x, l_y];
            uic.draw_text(graphics, l_pos, l_size, l_color, l_text);
        }
        // xy value string.
        let x_string = val_to_string(self.x, self.max_x,
                                     self.max_x - self.min_x, self.dim[0] as usize);
        let y_string = val_to_string(self.y, self.max_y,
                                     self.max_y - self.min_y, self.dim[1] as usize);
        let xy_string = format!("{}, {}", x_string, y_string);
        let xy_string_w = label::width(uic, self.font_size, &xy_string);
        let xy_string_pos = {
            match rectangle::corner(pad_pos, [vert_x, hori_y], pad_dim) {
                Corner::TopLeft => [vert_x, hori_y],
                Corner::TopRight => [vert_x - xy_string_w, hori_y],
                Corner::BottomLeft => [vert_x, hori_y - self.font_size as f64],
                Corner::BottomRight => [vert_x - xy_string_w, hori_y - self.font_size as f64],
            }
        };
        uic.draw_text(graphics, xy_string_pos, self.font_size,
                    color.plain_contrast(), &xy_string);

        set_state(uic, self.ui_id, Widget::XYPad(new_state), self.pos, self.dim);

    }
}
