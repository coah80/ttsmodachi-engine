const form = document.querySelector("#form");
const status = document.querySelector("#status");
const player = document.querySelector("#player");
const download = document.querySelector("#download");
const generate = document.querySelector("#generate");

const NUMBER_FIELDS = ["pitch", "speed", "quality", "tone", "accent", "intonation", "volume"];
let currentUrl = null;
let languages = [];

function applyPreset() {
  const language = languages.find((item) => item.id === form.elements.lang.value);
  if (!language) return;
  const preset = language.defaults[form.elements.preset.value];
  for (const [name, value] of Object.entries(preset)) {
    form.elements[name].value = value;
  }
  form.elements.text.maxLength = language.id === "jp" ? 1024 : 2000;
}

form.elements.lang.addEventListener("change", () => {
  form.elements.preset.value = "boy";
  applyPreset();
});
form.elements.preset.addEventListener("change", applyPreset);

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
  const voice = { engine: "tl3ds", lang: form.elements.lang.value };
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

async function loadLanguages() {
  status.textContent = "loading languages...";
  try {
    const response = await fetch("/languages");
    if (!response.ok) throw new Error(await errorText(response));
    ({ languages } = await response.json());
    if (!languages.length) throw new Error("no languages available");
    for (const language of languages) {
      form.elements.lang.add(new Option(language.name, language.id));
    }
    applyPreset();
    form.elements.lang.disabled = false;
    form.elements.preset.disabled = false;
    generate.disabled = false;
    status.textContent = "";
  } catch (error) {
    status.textContent = `failed: ${error.message}. check the worker and reload the page.`;
  }
}

loadLanguages();
