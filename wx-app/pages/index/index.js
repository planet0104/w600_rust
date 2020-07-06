//index.js
const app = getApp()

const client_id = "clientx11";

var connectServer = function(){
  console.log("connect...");
  wx.connectSocket({
    url: 'ws://192.168.1.7:9001'
  });
  wx.onSocketOpen((result) => {
    console.log("SocketOpen");
    //注册客户端id
    wx.sendSocketMessage({
      data: "wxapp",
    });
  });
  wx.onSocketClose((result) => {
    console.log('SocketClose');
  });
};

Page({
  data: {
    rgb: 'rgb(0,0,255)',
    pick: false
  },

  pickClose(){
    wx.closeSocket();
  },

  pickColor(e) {
    let rgb = e.detail.color;
    if(this.data.rgb != rgb){
      this.setData({ rgb });
      var v = rgb.replace("rgb(", "").replace(")", "").split(",");
      var msg = {
        data: JSON.stringify({
          target: client_id,
          r: v[0],
          g: v[1],
          b: v[2],
      })};
      console.log('send', msg);
      wx.sendSocketMessage(msg);
    }
  },

  connect(){
    connectServer(this);
    this.setData({pick: true});
  }
})
