export type DeviceInfo = {
	id: string;
	name: string;
	rows: number;
	columns: number;
	encoders: number;
	touchpoints: number;
	type: number;
};

export const SUPPORTED_DEVICE_LAYOUT_VERSION = 1;

export type DeviceRegistrationMetadata = {
	layoutVersion?: number;
	layout?: DeviceLayout;
};

export type DeviceDescriptor = DeviceInfo & DeviceRegistrationMetadata;

export type DeviceLayout = {
	canvas: DeviceLayoutCanvas;
	surfaces?: DeviceLayoutSurface[];
	controls: DeviceLayoutControl[];
};

export type DeviceLayoutCanvas = {
	width: number;
	height: number;
	unit?: string;
};

export type DeviceLayoutSurface = {
	id: string;
	kind?: "display" | "touchpanel";
	x: number;
	y: number;
	width: number;
	height: number;
	pixelWidth?: number;
	pixelHeight?: number;
};

export type DeviceLayoutControl = RectDeviceLayoutControl | CircleDeviceLayoutControl;

export type RectDeviceLayoutControl = {
	controller: string;
	position: number;
	shape: "rect";
	x: number;
	y: number;
	width: number;
	height: number;
	surface?: string;
};

export type CircleDeviceLayoutControl = {
	controller: string;
	position: number;
	shape: "circle";
	cx: number;
	cy: number;
	r: number;
	surface?: string;
};
