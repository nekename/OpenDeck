# Device Layouts

Plugins may optionally include explicit layout metadata when sending a `registerDevice` event. This is additive to the existing device registration payload; plugins that only send `rows`, `columns`, `encoders`, and `touchpoints` continue to use the legacy renderer.

OpenDeck currently supports `layoutVersion: 1`. Layout payloads with another version are ignored and the device falls back to the legacy renderer.

```json
{
  "event": "registerDevice",
  "payload": {
    "id": "FW-example-001",
    "name": "Example Device",
    "rows": 2,
    "columns": 4,
    "encoders": 2,
    "touchpoints": 1,
    "type": 0,
    "layoutVersion": 1,
    "layout": {
      "canvas": { "width": 520, "height": 320, "unit": "px" },
      "surfaces": [
        {
          "id": "keys",
          "kind": "display",
          "x": 80,
          "y": 40,
          "width": 360,
          "height": 180,
          "pixelWidth": 360,
          "pixelHeight": 180
        },
        {
          "id": "touch",
          "kind": "touchpanel",
          "x": 80,
          "y": 240,
          "width": 360,
          "height": 42,
          "pixelWidth": 800,
          "pixelHeight": 100
        }
      ],
      "controls": [
        {
          "controller": "Keypad",
          "position": 0,
          "shape": "rect",
          "x": 80,
          "y": 40,
          "width": 84,
          "height": 84,
          "surface": "keys"
        },
        {
          "controller": "Encoder",
          "position": 0,
          "shape": "circle",
          "cx": 122,
          "cy": 300,
          "r": 34
        }
      ]
    }
  }
}
```

The `canvas` defines the coordinate system used by `surfaces` and `controls`.

`surfaces` are optional display or touch-panel regions. Supported surface kinds are `display` and `touchpanel`. Display surfaces are metadata for grouping controls. Touch-panel surfaces are drawn so users can see the interactive region.

`controls` define the actual droppable controls. Supported controllers are `Keypad` and `Encoder`. Unsupported controllers are ignored by the current renderer.
