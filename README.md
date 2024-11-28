# Tacky-Borders

Bring sleek, customizable borders to your Windows desktop.

https://github.com/user-attachments/assets/246300e8-0853-4408-a05d-1b2ae91c4da0

## Installation

### Download Prebuilt Release
1. Visit the [Releases](https://github.com/GlazeBar/tacky-borders/releases) page.
2. Download your desired version.
3. Unzip the file and run the `.exe` file.

Upon the first run, `tacky-borders` will generate a configuration file located at ```%userprofile%/.config/tacky-borders/```.

### Build it Yourself
If you'd like to build the application manually:

1. Ensure you have the following tools installed:
    - [Rust](https://www.rust-lang.org/tools/install)
    - [MSVC](https://visualstudio.microsoft.com/downloads/)

2. Clone this repository:

    ```sh
    git clone https://github.com/GlazeBar/tacky-borders.git
    ```
    
3. Navigate into the project directory:

    ```sh
    cd tacky-borders
    ```
    
4. Build or run the project:

    ```sh
    cargo build --release
    ```

    or

    ```sh
    cargo run
    ```

## Configuration

Upon running `tacky-borders`, a configuration file is auto-generated with the following options:

- `border_width`: Thickness of the borders
- `border_offset`: Distance of borders from the window edges.
- `border_radius`:
    - Set to `-1` or `Auto` to let tacky-borders automatically adjust the radius.
    - Alternatively, specific your own value (numbers) or enum (Round, SmallRound, Square)
- `active_color`: Defines the color of the active window. Acceptable formats include: 
    - `accent`: Use the Windows accent color.
    - `rgba(r, g, b, a)` | `rgb(r, g, b)` | `#rrggbb` | `#rrggbbaa`: Use a custom solid color in RGBA, RGB, HEX or HEXA format.
    - `gradient(color1, color2, ..., direction)`: Use gradient color.
        - Colors can be in RGBA, RGB, HEX or HEXA format.
        - `direction` specifies the gradient's orientation (e.g., `to right`, `45deg`).
    - Alternatively, `active_color` can be written in a mapping or struct-like format:
        - `active_color: { colors: [...], direction: "to right" | "45deg" }`: A more detailed gradient format where `colors`
        contains the gradient colors, and `direction` specifies the gradient's orientation
        - `active_color: { colors: [...], direction: { start: [x, y], end: [x, y] } }`: A more advanced gradient format where
        `direction` is defined by the start and `end` coordinates for a custom directional gradient.
- `inactive_color`: Defines the color of the inactive window. Acceptable formats are the same as `active_color`
- `animations`: Defines the animations for the borders.
    - `active`: Defines the animation for active window borders. 
        - Format: `{ animation_type: speed (optional), ... }`
        - `animation_type`: Type of the animations (e.g., `Fade`, `Spiral`, etc.).
        - `speed` (optional): Speed of the animation
    - `inactive`: Defines the animation for inactive window borders. Format is the same as `animations.active`
    - `fps`: The number of frames per second for the animation 

Additionally, there are some optional config options that are not included in the auto-generated config file:

- `initialize_delay`: Delay (in milliseconds) between when a new window opens and when the border is displayed.
- `unminimize_delay`: Delay (in milliseconds) between when a window is restored/unminimized and when the border appears.

**Recommendation**: Set to 0 if Windows animations are disabled.

> **Note**: These delays are necessary to accommodate limitations with the Win32 API regarding window animations.

### Configuration Schema
To make customization easier, a [configuration schema](./schema.json) is available.

## Credits
This project makes use of the following open-source library:
- [Bezier-Easing](https://github.com/gre/bezier-easing) by Gaëtan Renaudeau

This is the original tacky-borders:
- [tacky-borders](https://github.com/lukeyou05/tacky-borders) by LukeYou05

## License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.
