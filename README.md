# Tacky-Borders

Bring sleek, customizable borders to your Windows desktop.

Use the original [tacky-borders](https://github.com/lukeyou05/tacky-borders) please!## Installation

https://github.com/user-attachments/assets/6520880d-b831-4652-b19d-34aa9b07f7cb

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

The config file is located in `%USERPROFILE%/.config/tacky-borders/`. You can easily modified the configuration by right clicking 
on the tray icon and hitting "Open Config" or by triggering `open_config` keybinding.

The following auto-generated `config.jsonc` is included as reference:

```json
{
  "$schema": "https://raw.githubusercontent.com/GlazeBar/tacky-borders/refs/heads/main/schema.json",
  // allow auto reload on config changes
  "monitor_config_changes": true,
  "theme": null, // Theme name from file in `%USERPROFILES%/.config/tacky-borders/themes`
  "keybindings": {
    // reload: Binds the reload action to the specified key (default: f8).
    // - Pressing this key will trigger the reloading process, typically refreshing content or settings.
    "reload": "f8",
    // open_config: Binds the action to open the configuration/settings to the specified key (default: f9).
    // - Pressing this key will open the configuration menu or editor settings for customization.
    "open_config": "f9",
    // exit: Binds the exit or quit action to the specified key (default: f10).
    // - Pressing this key will close the application or exit the current session.
    "exit": "f10"
  },
  "global": {
    // border_width: Specifies the thickness of the window border in pixels.
    //   - Example: 2 sets the border to 2 pixels wide.
    //   - You can also use string values like "2px".
    "border_width": 2, // Also accepts "2px"
    // border_offset: Adjusts the position of the border relative to the window.
    //   - Negative values shrink the border inward (reducing the visible area).
    //   - Positive values expand the border outward (increasing its coverage).
    //   - You can also use string values like "2px"
    // Example: -1 shrinks the border slightly inside the window.
    "border_offset": -1, // Also accepts "-1px"
    // border_style: Controls the shape of the window's border corners.
    //   - Use numeric values for custom corner rounding in pixels (e.g., 5 applies a 5-pixel radius).
    //   - Use predefined styles for consistent results:
    //     - "Auto": Automatically calculates a suitable radius based on the window's dimensions.
    //     - "Round": Fully rounded corners using a default radius.
    //     - "SmallRound": Smaller rounded corners with a default radius.
    //     - "Square": No rounding, resulting in square corners.
    //     - "Radius(num)": Applies a custom radius where "num" is a positive number defining the pixel radius of the corners.
    "border_style": "Auto", // or "Radius(10)" for custom 10px radius, or "Round", "SmallRound", etc.
    // active_color: Specifies the color properties for the active window border.
    //   - Acceptable formats:
    //     1. **String**:
    //        - Hex: "#RRGGBB" or "#RRGGBBAA".
    //        - CSS Functions: `rgb(...)` or `rgba(...)`.
    //        - Named Colors: Use predefined names from CSS (see: https://www.w3.org/TR/css-color-4/#named-colors).
    //        - Theme Reference: A color from the active theme.
    //     2. **Gradient Map**: Defines a linear gradient with the following properties:
    //        - `colors`: An array of color values (e.g., `["#89b4fa", "#cba6f7"]`).
    //        - `direction`: The start and end points of the gradient as normalized coordinates:
    //          - `start`: `[x, y]` (e.g., `[0.0, 0.0]`).
    //          - `end`: `[x, y]` (e.g., `[1.0, 0.0]`).
    //   - **Examples**:
    //     - Single Color:
    //       active_color: "#ff0000"
    //     - Gradient Map:
    //       active_color: {
    //         "colors": ["#89b4fa", "#cba6f7"],
    //         "direction": {
    //           "start": [0.0, 0.0],
    //           "end": [1.0, 0.0]
    //         }
    //       }
    "active_color": {
      "colors": [
        "#000000",
        "#ffffff"
      ],
      "direction": {
        "start": [
          0.0,
          0.0
        ],
        "end": [
          1.0,
          0.0
        ]
      }
    },
    //   - This can also be specified as a string or a gradient map, similar to active_color.
    "inactive_color": "#d2d2d2",
    // animations: Configures the animations applied to window borders.
    //   - active: Defines animations for active window transitions.
    //     - Each animation specifies its type (kind), duration, and easing function.
    //   - inactive: Defines animations for inactive window transitions.
    //     - Uses the same format as active animations.
    //   - fps: Sets the frame rate for animations (default: 60 FPS).
    // Example of an animation: { kind: "fade", duration: "450ms", easing: "EaseInOut" }
    "animations": {
      "active": [
        {
          "kind": "fade",
          "duration": "450ms",
          "easing": "EaseInOut"
        },
        {
          "kind": "spiral",
          "duration": "1800ms",
          "easing": "cubic-bezier(0.42, 0.0, 0.58, 1.0)"
        }
      ],
      "inactive": [
        {
          "kind": "fade",
          "duration": "450ms",
          "easing": "ease-in-out"
        },
        {
          "kind": "reverse_spiral",
          "duration": "1800ms",
          "easing": "cubic-bezier(0.42, 0.0, 0.58, 1.0)"
        }
      ],
      "fps": 60
    },
    // initialize_delay: The initial delay (in milliseconds) before applying animations when the window is first rendered. 
    //             A reduced delay can be used to account for animations like fade, which take additional time.
    // restore_delay: The delay (in milliseconds) before applying animations when a minimized window is restored.
    // Note: Recommended to make it 0 if there is no windows animation.
    "initialize_delay": 150,
    "restore_delay": 100
  },
  // window_rules: Defines specific window matching rules for borders.
  // Each rule can define custom properties for how borders are applied to matching windows.
  // The properties defined in window_rules can either inherit from the global settings or be overridden by the rule.
  "window_rules": [
    {
      // Match Strategies:
      // kind: Specifies the type of property to match.
      // - Process: Matches based on the process name.
      // - Title: Matches based on the window title.
      // - Class: Matches based on the window's class name.
      // strategy (default: Equals):
      // - "Equals": The match value must be exactly equal to the specified value.
      // - "Regex": The match value must match the specified regular expression.
      // - "Contains": The match value must be a substring of the specified string.
      // value:
      // - Specifies the value against which the window's properties are matched.
      // - Can be a string that must either match exactly, be a substring, or conform to a regex pattern
      //   depending on the specified match strategy.
      // enabled (default: true):
      // - A boolean value indicating whether the border is enabled for this particular rule.
      // - If true, the defined border properties (such as color and thickness) will apply to matching windows.
      // - If false, no border will be applied, effectively disabling it for that rule.
      "match": {
        "kind": "Class",
        "value": "Windows.UI.Core.CoreWindow",
        "strategy": "Contains",
        "enabled": false
      }
    },
    {
      "match": {
        "kind": "Process",
        "value": "(?i)^Flow.*",
        "strategy": "Regex",
        "enabled": false
      }
    },
    {
      "match": {
        "kind": "Title",
        "value": "Zebar",
        "strategy": "Equals",
        "enabled": false
      }
    }
  ]
}
```

### Configuration Schema
To make customization easier, a [configuration schema](./schema.json) is available.

### Theme Configuration Guide
To make defining colors easier, themes can be used in Tacky Borders. Themes allow you to use predefined color names instead of manually specifying colors each time. You can define colors in various formats, including:

- `#RGB`
- `#RRGGBB`
- `rgb(...)`
- `hsl(...)`

**Example Theme Format**
Below is an example of a theme file:

```json
{
    "rosewater": "#f5e0dc",
    "flamingo": "#f2cdcd",
    "pink": "#f5c2e7",
    "mauve": "#cba6f7",
    "red": "#f38ba8",
    "maroon": "#eba0ac",
    "peach": "#fab387",
    "yellow": "#f9e2af",
    "green": "#a6e3a1",
    "teal": "#94e2d5",
    "sky": "#89dceb",
    "sapphire": "#74c7ec",
    "blue": "#89b4fa",
    "lavender": "#b4befe",
    "text": "#cdd6f4",
    "subtext1": "#bac2de",
    "subtext0": "#a6adc8",
    "overlay2": "#9399b2",
    "overlay1": "#7f849c",
    "overlay0": "#6c7086",
    "surface2": "#585b70",
    "surface1": "#45475a",
    "surface0": "#313244",
    "base": "#1e1e2e",
    "mantle": "#181825",
    "crust": "#11111b"
}
```

#### Storing Themes
Place your theme files under the following directory for tacky-borders to recognize them:
```text
%USERPROFILE%/.config/tacky-borders/themes/{theme-name}.json(c)
```

## Credits
This project makes use of the following open-source library:
- [Bezier-Easing](https://github.com/gre/bezier-easing) by Gaëtan Renaudeau

## License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.
