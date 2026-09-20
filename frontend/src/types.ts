export interface User {
    id: string;
    email: string;
    full_name: string;
    is_active: boolean;
    is_superuser: boolean;
}

export interface Item {
    id: string;
    title: string;
    description: string;
    owner_id: string;
    quantity: number;
    created_at: string;
}

export type WorkspaceView = "overview" | "items" | "users";
