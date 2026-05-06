const loader = document.getElementById("bevy-loader");
const progress = document.getElementById("bevy-progress");

function setState(state) {
  loader?.setAttribute("data-state", state);
}

function setProgress(current, total) {
  if (!progress) {
    return;
  }

  if (!total) {
    progress.textContent = "Loading demo...";
    return;
  }

  const percent = Math.max(0, Math.min(100, Math.round((current / total) * 100)));
  progress.textContent = `Loading demo... ${percent}%`;
}

export default function bevyInitializer() {
  return {
    onStart() {
      setState("loading");
      setProgress(0, 0);
    },
    onProgress({ current, total }) {
      setProgress(current, total);
    },
    onSuccess() {
      setState("ready");
    },
    onFailure(error) {
      console.error(error);
      setState("error");
    },
  };
}
