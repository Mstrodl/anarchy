Build wasm package:
```
# in `anarchy_web`
wasm-pack build --release
```

Update newly built package on frontend:
```
# in `wwww`
pnpm i
```

Run frontend:
```
# in `www`
NODE_ENV=production pnpm start
```
