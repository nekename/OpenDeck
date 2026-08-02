# Touch swipe plugin event

OpenDeck exposes swipe gestures performed on the Stream Deck Plus
touchscreen through the `touchSwipe` plugin event.

This event is an OpenDeck extension and is not currently part of the
official Stream Deck plugin protocol.

## Event payload

```json
{
  "event": "touchSwipe",
  "action": "com.example.plugin.action",
  "context": "action-instance-context",
  "device": "sd-device-serial",
  "payload": {
    "controller": "Encoder",
    "settings": {},
    "coordinates": {
      "row": 0,
      "column": 2
    },
    "profile": "Audio",
    "startPos": [568, 72],
    "endPos": [640, 66],
    "delta": [72, -6],
    "direction": "right"
  }
}```
