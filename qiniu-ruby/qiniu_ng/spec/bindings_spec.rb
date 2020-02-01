RSpec.describe QiniuNg::Bindings do
  context QiniuNg::Bindings::Str do
    it 'should be ok to initialize string' do
      str1 = QiniuNg::Bindings::Str.new '你好'
      str2 = QiniuNg::Bindings::Str.new '七牛'
      expect(str1.get_ptr).to eq('你好')
      expect(str2.get_ptr).to eq('七牛')
      expect(str1.get_len).to eq('你好'.bytesize)
      expect(str2.get_len).to eq('七牛'.bytesize)
      expect(str1.is_freed?).to be false
      expect(str2.is_freed?).to be false
      expect(str1.is_null?).to be false
      expect(str2.is_null?).to be false
    end
  end

  context QiniuNg::Bindings::StrList do
    it 'should be ok to initialize string list' do
      list1 = QiniuNg::Bindings::StrList.new(['七牛', '你好', '武汉', '加油'])
      list2 = QiniuNg::Bindings::StrList.new(['科多兽', '多啦A梦', '潘多拉'])
      expect(list1.len).to eq(4)
      expect(list2.len).to eq(3)
      expect(list1.get(0)).to eq('七牛')
      expect(list1.get(1)).to eq('你好')
      expect(list1.get(2)).to eq('武汉')
      expect(list1.get(3)).to eq('加油')
      expect(list2.get(0)).to eq('科多兽')
      expect(list2.get(1)).to eq('多啦A梦')
      expect(list2.get(2)).to eq('潘多拉')
      expect(list1.is_freed?).to be false
      expect(list2.is_freed?).to be false
    end
  end
end
