const { get, API_URL } = require("./client");

async function main() {
  console.log("{{project_name}} — powered by nucleo");
  console.log(`API URL: ${API_URL}`);

  try {
    const data = await get("https://httpbin.org/get");
    console.log("Response:", JSON.stringify(data, null, 2));
  } catch (err) {
    console.error("Request failed:", err.message);
  }
}

main();
