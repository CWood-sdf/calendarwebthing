setTimeout(() => {
  var els = document.querySelectorAll(".time");
  var arr = [];
  for (var i = 0; i < els.length; i++) {
    arr.push(els[i]);
  }
  els = arr;
  console.log(els.map((e) => e.textContent));
  els.forEach((el) => {
    if (!el.textContent.match(/^[0-9]+$/)) return;
    el.textContent = new Date(parseInt(el.textContent) * 1000).toLocaleString();
  });
});
