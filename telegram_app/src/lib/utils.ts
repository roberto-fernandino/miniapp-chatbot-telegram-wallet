import { type ClassValue, clsx } from "clsx"
import axios from "axios"
import { twMerge } from "tailwind-merge"
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

type UserData = {
  id: number;
  first_name: string;
  last_name: string;
  username: string;
  language_code: string;
  allows_write_to_pm: boolean;
  wallets_id: string[] | null;
}

export function parseUserData(userData: any) {
  return {
    id: userData.id.toString(),
    first_name: userData.first_name,
    last_name: userData.last_name,
    username: userData.username,
    language_code: userData.language_code,
    allows_write_to_pm: userData.allows_write_to_pm,
    wallets_id: userData.wallets_id,
  }
}

// Add or update user in redis
export async function addOrUpdateUser(userData: UserData) {
  try {
    const response = await axios.post("http://selected-namely-panda.ngrok-free.app/api/add_or_update_user", userData, {
      headers: {
        'Content-Type': 'application/json'
      }
    });
    return response;
  } catch (error) {
    throw error; // Re-throw the error so it can be caught in the calling function
  }
}