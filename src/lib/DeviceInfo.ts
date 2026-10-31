export type DeviceSlot = {
	/** The position of the slot in the device's grid. */
	position: number;
	/** Whether the slot is round, like a dial. */
	round: boolean;
	/** The index of the rotary encoder occupying the slot, if any. */
	encoder: number | null;
};

/** How the rotary encoders are laid out in their row. */
export type EncoderAlignment = "between" | "start" | "center" | "end";

export type DeviceInfo = {
	id: string;
	name: string;
	rows: number;
	columns: number;
	encoders: number;
	touchpoints: number;
	infobars: number;
	type: number;
	/** How the rotary encoders are laid out in their row. */
	encoder_alignment: EncoderAlignment | null;
	/** The physical controls that occupy the device's grid. */
	slots: DeviceSlot[] | null;
};
