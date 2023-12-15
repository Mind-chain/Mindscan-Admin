import fetch from "node-fetch";

export const makePostRequest = async (url: string, data: any, headers: any) => {
  return await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...headers,
    },
    body: JSON.stringify(data),
  });
};
