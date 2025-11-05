#### windows

> windows msi 编译目前依赖 wix 6

```shell
# 软链接 ui代码到指定位置
cmd /c mklink /J ui $UI 
```


#### cloudflare works

```shell
cd crates/binary-works-cloudflare
# 手动更新配置
npx wrangler secret put share --env prod
```