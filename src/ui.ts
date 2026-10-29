// In-window notices and questions, drawn in the app's own pixel style instead
// of the browser's alert() and confirm() boxes (which show "localhost says").

let toastTimer: number | undefined;

/**
 * Fills a live panel (the windows refresh every second or two). An unchanged
 * panel is left alone, so its canvases keep their pictures and what you were
 * looking at stays put; when it does change, the lists you opened
 * (`<details data-key>`) stay open. Returns whether it changed.
 */
export function render(target: string | HTMLElement, html: string): boolean {
  const el = typeof target === "string" ? document.getElementById(target)! : target;
  if (el.dataset.rendered === html) return false;
  const open = new Set([...el.querySelectorAll<HTMLDetailsElement>("details[open]")].map((d) => d.dataset.key));
  el.innerHTML = html;
  el.dataset.rendered = html;
  for (const d of el.querySelectorAll<HTMLDetailsElement>("details")) d.open = open.has(d.dataset.key);
  return true;
}

/**
 * A short notice that slides up at the bottom of the window and fades away.
 * Errors stay a little longer. Screen readers announce it.
 */
export function toast(message: string, kind: "info" | "ok" | "error" = "info"): void {
  let el = document.getElementById("ui-toast");
  if (!el) {
    el = document.createElement("div");
    el.id = "ui-toast";
    el.className = "ui-toast px-box";
    el.setAttribute("role", "status");
    el.setAttribute("aria-live", "polite");
    el.addEventListener("click", () => el!.classList.remove("show"));
    document.body.append(el);
  }
  el.textContent = message;
  el.dataset.kind = kind;
  // Restart the slide-in even if a notice is already showing.
  el.classList.remove("show");
  void el.offsetWidth;
  el.classList.add("show");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => el!.classList.remove("show"), kind === "error" ? 6000 : 3500);
}

/** "Could not …" as a notice, for a failed action. */
export function failed(what: string, err: unknown): void {
  toast(`Could not ${what}: ${err}`, "error");
}

/**
 * Asks a yes/no question in a pixel dialog over the window. Resolves true for
 * yes; Escape, the no button or a click outside mean no.
 */
export function ask(question: string, yes = "Yes", no = "Cancel"): Promise<boolean> {
  return new Promise((resolve) => {
    const dialog = document.createElement("dialog");
    dialog.className = "ui-ask px-box";
    dialog.innerHTML = `<p></p><div class="ui-ask-buttons"><button class="secondary" value="no"></button><button value="yes"></button></div>`;
    dialog.querySelector("p")!.textContent = question;
    const [noButton, yesButton] = dialog.querySelectorAll("button");
    noButton.textContent = no;
    yesButton.textContent = yes;
    const done = (answer: boolean) => {
      dialog.close();
      dialog.remove();
      resolve(answer);
    };
    yesButton.addEventListener("click", () => done(true));
    noButton.addEventListener("click", () => done(false));
    dialog.addEventListener("cancel", (e) => {
      e.preventDefault();
      done(false);
    });
    // A click on the dimmed backdrop (outside the box) is a no.
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) done(false);
    });
    document.body.append(dialog);
    dialog.showModal();
    yesButton.focus();
  });
}
