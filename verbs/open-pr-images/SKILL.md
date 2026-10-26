---
name: open-pr-images
description: O bipri. Abre ou atualiza um PR bilíngue como o verbo open-pr, com imagens de antes e depois no corpo. Use em todo PR que muda uma imagem, um desenho, uma página ou uma tela, e quando pedirem /bipri ou um PR com imagens.
---

# open-pr-images

O `open-pr-images` (o bipri) é o verbo `open-pr` (o bipr) com imagens. Siga todos os passos do
`open-pr`, nesta ordem, e acrescente os passos abaixo. Uma mudança sem efeito visível usa o
`open-pr`.

## 1. Capture o antes e o depois

1. Renderize o estado do branch base (`origin/<base>`) e o estado do HEAD, com o mesmo tamanho e
   o mesmo fundo. Use o Chromium do Playwright para SVG, HTML e páginas.
2. As imagens são temporárias: `before.png`, `after.png` e, quando a imagem prova uma regra (uma
   medida, uma grade, uma proporção), uma terceira imagem com essa prova.
3. Nunca faça o commit dessas imagens no branch do PR. Faça o commit delas no branch
   `pr-images/<número do PR>`, que nunca entra no `main`. O corpo do PR aponta para esse branch.

## 2. Escreva o corpo

Use o corpo do `open-pr` e acrescente a seção **Imagens** nas duas metades, logo depois de **O que
mudou** e de **What changed**:

```
**Imagens**

| Antes | Depois |
|---|---|
| ![antes](<url de before.png>) | ![depois](<url de after.png>) |

<uma frase: o que a imagem de prova mostra>
```

```
**Images**

| Before | After |
|---|---|
| ![before](<url of before.png>) | ![after](<url of after.png>) |

<one sentence: what the proof image shows>
```

- A URL de uma imagem é `https://github.com/<owner>/<repo>/blob/pr-images/<número do PR>/<caminho>?raw=true`. Essa
  URL abre num repositório privado para quem tem acesso.
- As duas metades mostram as mesmas imagens. Só o texto muda de idioma.
- A imagem não substitui o texto: o **Resumo** diz o que mudou sem a imagem.

## 3. Rodapé

O rodapé é o `footer.md` do verbo `open-pr`, sem mudança.
