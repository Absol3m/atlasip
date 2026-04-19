let open = $state(false);

export const aboutModal = {
  get open() { return open; },
  show() { open = true; },
  hide() { open = false; },
};
