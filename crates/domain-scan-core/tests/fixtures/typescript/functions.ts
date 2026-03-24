// Test fixture: TypeScript functions

export function add(a: number, b: number): number {
  return a + b;
}

export async function fetchUser(id: string): Promise<User> {
  const response = await fetch(`/api/users/${id}`);
  return response.json();
}

function privateHelper(data: string): string {
  return data.trim();
}

export const multiply = (a: number, b: number): number => a * b;

export const fetchData = async (url: string): Promise<Response> => {
  return fetch(url);
};

const internalTransform = (items: string[]): string[] => {
  return items.map(i => i.toLowerCase());
};

export const processEvent = function(event: Event): void {
  // process
};
