/* CollabBoard Portfolio — Vanilla JS */
(function () {
  'use strict';

  var prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /* --- Nav Page Switching --- */
  var navButtons = document.querySelectorAll('.nav-item[data-page]');
  var pages = document.querySelectorAll('.page[data-page]');

  function switchPage(pageName) {
    navButtons.forEach(function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-page') === pageName);
      if (btn.getAttribute('data-page') === pageName) {
        btn.setAttribute('aria-current', 'page');
      } else {
        btn.removeAttribute('aria-current');
      }
    });
    pages.forEach(function (page) {
      page.classList.toggle('active', page.getAttribute('data-page') === pageName);
    });
  }

  navButtons.forEach(function (btn) {
    btn.addEventListener('click', function () {
      switchPage(btn.getAttribute('data-page'));
    });
  });

  /* --- Hamburger Menu --- */
  var hamburger = document.querySelector('.nav-hamburger');
  var navBar = document.querySelector('.nav-bar');

  if (hamburger) {
    hamburger.addEventListener('click', function () {
      var isOpen = navBar.classList.toggle('nav-open');
      hamburger.setAttribute('aria-expanded', isOpen);
    });

    /* Close menu when a nav item is clicked on mobile */
    navButtons.forEach(function (btn) {
      btn.addEventListener('click', function () {
        navBar.classList.remove('nav-open');
        hamburger.setAttribute('aria-expanded', 'false');
      });
    });
  }

  /* --- Tab Switching --- */
  var tabBars = document.querySelectorAll('.tab-bar');

  tabBars.forEach(function (bar) {
    var tabs = bar.querySelectorAll('.tab');
    tabs.forEach(function (tab) {
      tab.addEventListener('click', function () {
        var targetId = tab.getAttribute('data-tab');

        /* Deactivate siblings */
        tabs.forEach(function (t) { t.classList.remove('active'); });
        tab.classList.add('active');

        /* Find closest page or record-card container to scope content lookup */
        var container = bar.closest('.record-card') || bar.closest('.page');
        if (!container) return;

        var contents = container.querySelectorAll('.tab-content');
        contents.forEach(function (tc) {
          tc.classList.toggle('active', tc.getAttribute('data-tab-content') === targetId);
        });
      });
    });
  });

  /* --- Stat Counter Animation --- */
  function animateCounters() {
    var statNumbers = document.querySelectorAll('.stat-number[data-target]');
    var duration = prefersReducedMotion ? 0 : 600;

    statNumbers.forEach(function (el) {
      var target = parseFloat(el.getAttribute('data-target'));
      var suffix = el.getAttribute('data-suffix') || '';
      var decimals = parseInt(el.getAttribute('data-decimals') || '0', 10);

      if (duration === 0) {
        el.textContent = formatNumber(target, decimals) + suffix;
        return;
      }

      var start = performance.now();

      function step(now) {
        var elapsed = now - start;
        var progress = Math.min(elapsed / duration, 1);
        var current = target * progress;
        el.textContent = formatNumber(current, decimals) + suffix;
        if (progress < 1) {
          requestAnimationFrame(step);
        }
      }

      requestAnimationFrame(step);
    });
  }

  function formatNumber(num, decimals) {
    if (decimals > 0) {
      return num.toFixed(decimals);
    }
    return Math.round(num).toLocaleString('en-US');
  }

  animateCounters();

  /* --- Timeline Carousel --- */
  var carousel = document.getElementById('timeline-carousel');

  if (carousel) {
    var slides = carousel.querySelectorAll('.carousel-slide');
    var prevBtn = carousel.querySelector('.carousel-prev');
    var nextBtn = carousel.querySelector('.carousel-next');
    var pagination = document.getElementById('timeline-pagination');
    var overlayTitle = document.getElementById('timeline-overlay-title');
    var overlayDate = document.getElementById('timeline-overlay-date');
    var currentSlide = 0;

    var dayData = [
      { title: 'DAY 1 \u2014 PROJECT BOOTSTRAP', date: '2026-02-14' },
      { title: 'DAY 2 \u2014 MVP DRAWING', date: '2026-02-15' },
      { title: 'DAY 3 \u2014 CANVAS ENGINE', date: '2026-02-16' },
      { title: 'DAY 4 \u2014 AI INTEGRATION', date: '2026-02-17' },
      { title: 'DAY 5 \u2014 EARLY RELEASE', date: '2026-02-18' },
      { title: 'DAY 6 \u2014 OBSERVABILITY', date: '2026-02-19' },
      { title: 'DAY 7 \u2014 FINAL POLISH', date: '2026-02-20' }
    ];

    function showSlide(index) {
      slides.forEach(function (s, i) {
        s.classList.toggle('active', i === index);
      });
      currentSlide = index;
      if (pagination) {
        pagination.textContent = 'DAY ' + (index + 1) + ' OF 7';
      }
      if (overlayTitle && dayData[index]) {
        overlayTitle.textContent = dayData[index].title;
      }
      if (overlayDate && dayData[index]) {
        overlayDate.textContent = dayData[index].date;
      }
    }

    if (prevBtn) {
      prevBtn.addEventListener('click', function () {
        showSlide((currentSlide - 1 + slides.length) % slides.length);
      });
    }

    if (nextBtn) {
      nextBtn.addEventListener('click', function () {
        showSlide((currentSlide + 1) % slides.length);
      });
    }

    /* Keyboard navigation for carousel */
    document.addEventListener('keydown', function (e) {
      var timelinePage = document.querySelector('.page[data-page="timeline"]');
      if (!timelinePage || !timelinePage.classList.contains('active')) return;

      if (e.key === 'ArrowLeft') {
        prevBtn && prevBtn.click();
      } else if (e.key === 'ArrowRight') {
        nextBtn && nextBtn.click();
      }
    });
  }
})();
