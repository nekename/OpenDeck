export type IconPackAvatar = { type: "dataurl"; url: string } | { type: "fs"; path: string };

export type IconPack = {
    id: string;
    name: string;
    author: string;
    version: string;
    icon: IconPackAvatar;
    installed_path?: string;
};
