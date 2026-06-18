import type { DeviceDescriptor } from "$lib/DeviceInfo";
import { SUPPORTED_DEVICE_LAYOUT_VERSION } from "$lib/DeviceInfo";

const LEGACY_LAYOUT_CELL_SIZE_PX = 132;

export function getDeviceCanvasSize(device: DeviceDescriptor) {
	if (device.layoutVersion == SUPPORTED_DEVICE_LAYOUT_VERSION && device.layout) {
		return {
			width: device.layout.canvas.width,
			height: device.layout.canvas.height,
		};
	}

	const columns = Math.max(device.columns, device.encoders, device.touchpoints, 1);
	const rows = device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1);
	return {
		width: columns * LEGACY_LAYOUT_CELL_SIZE_PX,
		height: Math.max(rows, 1) * LEGACY_LAYOUT_CELL_SIZE_PX,
	};
}
