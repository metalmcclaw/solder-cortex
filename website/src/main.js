import './style.css'

// Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
  anchor.addEventListener('click', function (e) {
    e.preventDefault()
    const target = document.querySelector(this.getAttribute('href'))
    if (target) {
      target.scrollIntoView({
        behavior: 'smooth',
        block: 'start'
      })
    }
  })
})

// Add scroll effect to nav
const nav = document.querySelector('.nav')
let lastScroll = 0

window.addEventListener('scroll', () => {
  const currentScroll = window.pageYOffset
  
  if (currentScroll > 100) {
    nav.style.background = 'rgba(10, 10, 15, 0.95)'
  } else {
    nav.style.background = 'rgba(10, 10, 15, 0.8)'
  }
  
  lastScroll = currentScroll
})

// Animate elements on scroll
const observerOptions = {
  threshold: 0.1,
  rootMargin: '0px 0px -50px 0px'
}

const observer = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      entry.target.classList.add('animate-in')
      observer.unobserve(entry.target)
    }
  })
}, observerOptions)

// Observe feature cards, problem cards, and other animated elements
document.querySelectorAll('.feature-card, .problem-card, .example-card').forEach(el => {
  el.style.opacity = '0'
  el.style.transform = 'translateY(20px)'
  el.style.transition = 'opacity 0.6s ease, transform 0.6s ease'
  observer.observe(el)
})

// Add animation class styles
const style = document.createElement('style')
style.textContent = `
  .animate-in {
    opacity: 1 !important;
    transform: translateY(0) !important;
  }
`
document.head.appendChild(style)

// Stagger animation for grid items
document.querySelectorAll('.features-grid, .solution-grid, .examples-grid').forEach(grid => {
  const items = grid.querySelectorAll('.feature-card, .problem-card, .example-card')
  items.forEach((item, index) => {
    item.style.transitionDelay = `${index * 0.1}s`
  })
})

// Console easter egg
console.log(`
🤘 Solder Cortex
Cross-Domain Intelligence for Solana

Built for Colosseum Agent Hackathon

https://github.com/metalmcclaw/solder-cortex
`)
