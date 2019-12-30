document.addEventListener('DOMContentLoaded', () => {

  function getAll(selector) {
    return Array.prototype.slice.call(document.querySelectorAll(selector), 0);
  }

  // Get all "navbar-burger" elements
  const $navbarBurgers = getAll('.navbar-burger');

  // Check if there are any navbar burgers
  if ($navbarBurgers.length > 0) {

    // Add a click event on each of them
    $navbarBurgers.forEach( el => {
      el.addEventListener('click', () => {

        // Get the target from the "data-target" attribute
        const target = el.dataset.target;
        const $target = document.getElementById(target);

        // Toggle the "is-active" class on both the "navbar-burger" and the "navbar-menu"
        el.classList.toggle('is-active');
        $target.classList.toggle('is-active');

      });
    });
  }

  var $dropdowns = getAll('.dropdown:not(.is-hoverable)');

  if ($dropdowns.length > 0) {
    $dropdowns.forEach(function ($el) {
      $el.addEventListener('click', function (event) {
        event.stopPropagation();
        $el.classList.toggle('is-active');
      });
    });
  }

  document.addEventListener('click', function (event) {
    closeDropdowns();
  });
  document.addEventListener("keydown", event => {
    if (event.isComposing || event.keyCode === 27) {
      closeDropdowns();
    }
  });

  function closeDropdowns() {
    var $dropdowns = getAll('.dropdown:not(.is-hoverable)'); // TODO improve
    $dropdowns.forEach(function ($el) {
      $el.classList.remove('is-active');
    });
  }
});

function dropdown_click(event)
{
  event.stopPropagation();
  event.currentTarget.classList.toggle('is-active');
}