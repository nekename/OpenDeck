export type Context = {
	device: string;
	profile: string;
	page: number;
	controller: string;
	position: number;
};

export function actionContextString(context: Context, index = 0): string {
	return `${context.device}.${context.profile}.${context.page ?? 0}.${context.controller}.${context.position}.${index}`;
}

export function parseActionContext(value: string): { device: string; profile: string; page: number; controller: string; position: number; index: number } {
	const segments = value.split(".");
	if (segments.length >= 6) {
		return {
			device: segments[0],
			profile: segments[1],
			page: parseInt(segments[2]),
			controller: segments[3],
			position: parseInt(segments[4]),
			index: parseInt(segments[5]),
		};
	}
	return {
		device: segments[0],
		profile: segments[1],
		page: 0,
		controller: segments[2],
		position: parseInt(segments[3]),
		index: parseInt(segments[4]),
	};
}
