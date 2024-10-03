# Okruženje

Što se tiče razvojnog okruženja potrebno je opisati razvojne ovisnosti i
razvojni proces. Ove ovisnosti su potrebne za izvršavanje naredbi definiranih u
`justfile` koji opisuje razvojni proces pobliže.

## Ovisnosti

- **Rust**: Projekt koristi Rust, a naredba `cargo` se koristi za izgradnju,
  testiranje i pokretanje Rust koda. Također se koristi za generiranje
  dokumentacije i formatiranje Rust koda.
- **Docker**: Docker se koristi za upravljanje uslugama o kojima aplikacija
  ovisi. Naredba `docker compose up -d` se koristi za pokretanje tih usluga, a
  `docker compose down -v` se koristi za njihovo zaustavljanje.

## Opcionalne ovisnosti

Sljedeći alati su opcionalni za neke razvojne procese, ali se preporučuju za
razvoj:

### Probe

- **Python**: Python se koristi za `probe` skriptu. Trebate imati instaliran
  Python kako biste pokrenuli ovu skriptu.
- **Poetry**: Poetry se koristi za upravljanje Python ovisnostima.

### Formatiranje

- **Yapf**: Yapf se koristi za formatiranje Python koda u projektu.
- **Prettier**: Prettier se koristi za formatiranje i provjeru formata koda u
  projektu.
- **shfmt**: shfmt se koristi za formatiranje shell skripti u projektu.

### Provjera koda

- **ShellCheck**: ShellCheck se koristi za provjeru shell skripti.
- **cspell**: cspell se koristi za provjeru pravopisa u projektu.
- **Ruff**: Ruff se koristi za provjeru Rust koda u projektu.
- **Clippy**: Clippy je Rust linter koji se koristi u projektu.
- **Pyright**: Pyright se koristi za provjeru tipova Python koda.

### Dokumentacija

- **mdbook**: mdbook se koristi za izradu dokumentacije u `docs` direktoriju.

### Stress testiranje

- **GNS3**: GNS3 se koristi za simuliranje mreže.

## Razvojni proces

Razvojni proces upravlja se pomoću `just`, upravitelja naredbama sličnog `make`.
`justfile` definira različite naredbe za izgradnju, testiranje, pokretanje i
upravljanje projektom.

Evo koraka za postavljanje razvojnog okruženja i korištenje `just`:

1. **Instalirajte ovisnosti**: Instalirajte sve potrebne alate navedene u ovom
   poglavlju.

2. **Pripremite okruženje**: Pokrenite `just prepare` za instalaciju Python
   ovisnosti, pokretanje Docker usluga i pokretanje migracija baze podataka.

3. **Pokrenite aplikaciju**: Koristite `just run` za pokretanje aplikacije.
   Možete proslijediti argumente aplikaciji dodavanjem uz naredbu, poput
   `just run --arg`.

4. **Pokrenite Probe skriptu**: Koristite `just probe` za pokretanje probe
   skripte. Možete proslijediti argumente skripti na isti način kao i za run
   naredbu.

5. **Formatirajte kod**: Koristite `just format` za formatiranje koda u projektu
   pomoću raznih formatera.

6. **Provjerite kod**: Koristite `just lint` za provjeru koda u projektu pomoću
   raznih lint alata.

7. **Testirajte kod**: Koristite `just test` za pokretanje testova za projekt.

8. **Izgradite projekt**: Koristite `just build` za izgradnju projekta. Ovo će
   stvoriti release build projekta i premjestiti rezultat u `artifacts`
   direktorij.

9. **Generirajte dokumentaciju**: Koristite `just docs` za generiranje
   dokumentacije projekta. Ovo će izgraditi dokumentaciju i premjestiti rezultat
   u `artifacts` direktorij.

Zapamtite da pokrenete `just prepare` svaki put kada povučete nove promjene iz
repozitorija kako biste osigurali da je vaše okruženje ažurirano.
