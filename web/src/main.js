const form = document.querySelector("#form");
const status = document.querySelector("#status");
const player = document.querySelector("#player");
const download = document.querySelector("#download");
const generate = document.querySelector("#generate");

const NUMBER_FIELDS = ["pitch", "speed", "quality", "tone", "accent", "intonation", "volume"];
let currentUrl = null;

async function errorText(response) {
  const body = await response.text();
  try {
    const { detail } = JSON.parse(body);
    return typeof detail === "string" ? detail : JSON.stringify(detail);
  } catch {
    return body || `HTTP ${response.status}`;
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const voice = { engine: "tl3ds", lang: "useng" };
  for (const name of NUMBER_FIELDS) {
    voice[name] = Number(form.elements[name].value);
  }
  generate.disabled = true;
  status.textContent = "generating...";
  try {
    const response = await fetch("/render", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text: form.elements.text.value, voice, mode: "text" }),
    });
    if (!response.ok) {
      throw new Error(await errorText(response));
    }
    if (currentUrl) {
      URL.revokeObjectURL(currentUrl);
    }
    currentUrl = URL.createObjectURL(await response.blob());
    player.src = currentUrl;
    download.href = currentUrl;
    download.hidden = false;
    status.textContent = "";
    await player.play().catch(() => {});
  } catch (error) {
    status.textContent = `failed: ${error.message}`;
  } finally {
    generate.disabled = false;
  }
});
